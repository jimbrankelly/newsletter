use actix_web::{web, App, HttpServer};
//use actix_web::middleware::Logger;
use tracing_actix_web::TracingLogger;
use actix_web::dev::Server;
use std::net::TcpListener;
use sqlx::PgPool;

use crate::routes::{health_check, subscribe};
use crate::email_client::EmailClient;

pub fn run(
    listener: TcpListener,
    connection_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    // Wrap the connection in a smart pointer
    let connection = web::Data::new(connection_pool);
    let email_client = web::Data::new(email_client);

    let server = HttpServer::new(move || 
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
            .app_data(email_client.clone())
    )
    .listen(listener)?
    .run();

    Ok(server)
}
