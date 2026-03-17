use newsletter::startup::run;
use newsletter::{
    configuration::get_configuration,
    telemetry::{init_subscriber, get_subscriber,},
    email_client::EmailClient,
};
use sqlx::postgres::PgPoolOptions;

use std::net::TcpListener;
//use sqlx::PgPool;
//use secrecy::ExposeSecret;

#[tokio::main]
async fn main() -> std::io::Result<()> {

    let subscriber = get_subscriber(
        "newsletter".into(), 
        "info".into(),
        std::io::stdout,
    );
    init_subscriber(subscriber);

    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration.");

    /*let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(
            &configuration.database.connection_string().expose_secret()
        )
        .expect("Failed to connect to Postgres.");*/
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        // `connect_lazy_with` instead of `connect_lazy`
        .connect_lazy_with(configuration.database.with_db());

    let _ = sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to migrate db: {:?}", e);
            ()
        } );

    // Build an `EmailClient` using `configuration`
    let sender_email = configuration.email_client.sender()
        .expect("Invalid sender email address.");
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
    );

    let address = format!(
        "{}:{}", 
        configuration.application.host,
        configuration.application.port
    );

    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool, email_client)?.await
}


/*use actix_web::{
    web, App, HttpRequest, HttpServer, Responder,
    HttpResponse,
};

async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!\n", &name)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(greet))
            .route("/health_check", web::get().to(health_check))
            .route("/{name}", web::get().to(greet))
        })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}

async fn health_check( ) -> impl Responder {
    HttpResponse::Ok().finish()
}*/
