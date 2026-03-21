use actix_web::{
    App, HttpServer,
    middleware::from_fn,
    web, web::Data, 
};
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web_flash_messages::{
    FlashMessagesFramework, 
    storage::CookieMessageStore, 
};
use actix_session::{SessionMiddleware, storage::RedisSessionStore, };
//use actix_web::middleware::Logger;
use tracing_actix_web::TracingLogger;
use std::net::TcpListener;
use sqlx::{PgPool, postgres::PgPoolOptions,};
use secrecy::{SecretString, ExposeSecret, };

use crate::{
    authentication::reject_anonymous_users,
    configuration::{Settings, DatabaseSettings, },
    email_client::EmailClient,
    routes::{
        admin_dashboard,
        change_password, change_password_form,
        confirm, health_check, home, 
        login_form, login, log_out,
        publish_newsletter, subscribe, 
    },
};

// A new type to hold the newly built server and its port 
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
  pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
      let connection_pool = get_connection_pool(&configuration.database);

      //TODO: how to handle for tests and for main?
      let _ = sqlx::migrate!("./migrations")
          .run(&connection_pool)
          .await
          .unwrap_or_else(|e| {
              tracing::warn!("Failed to migrate db: {:?}", e);
              ()
          } );

      let sender_email = configuration
          .email_client
          .sender()
          .expect("Invalid sender email address.");
      let timeout = configuration.email_client.timeout();
      let email_client = EmailClient::new(
          configuration.email_client.base_url,
          sender_email,
          configuration.email_client.authorization_token,
          timeout,
      );

      let address = format!(
          "{}:{}",
          configuration.application.host, configuration.application.port
      );
      let listener = TcpListener::bind(address)?;
      let port = listener.local_addr().unwrap().port();
      let server = run(
        listener, 
        connection_pool, 
        email_client,
        configuration.application.base_url,
        configuration.application.hmac_secret,
        configuration.redis_uri
      ).await?;

        // We "save" the bound port in one of `Application`'s fields
        Ok(Self { port, server })
  }

  pub fn port(&self) -> u16 {
      self.port
  }

  // A more expressive name that makes it clear that 
  // this function only returns when the application is stopped.
  pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
      self.server.await
  }
}

pub fn get_connection_pool(
    configuration: &DatabaseSettings
) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

// We need to define a wrapper type in order to retrieve the URL
// in the `subscribe` handler. 
// Retrieval from the context, in actix-web, is type-based: using
// a raw `String` would expose us to conflicts.
pub struct ApplicationBaseUrl(pub String);

pub async fn run(
    listener: TcpListener,
    connection_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: SecretString,
    redis_uri: SecretString,
) -> Result<Server, anyhow::Error> {
    // Wrap the connection in a smart pointer
    let connection = Data::new(connection_pool);
    let email_client = Data::new(email_client);
    let base_url = Data::new(ApplicationBaseUrl(base_url));

    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let message_store = CookieMessageStore::builder(
        secret_key.clone()
    ).build();
    let message_framework = FlashMessagesFramework::builder(
        message_store).build();
    let redis_store = RedisSessionStore::new(
        redis_uri.expose_secret()
    ).await?;

    let server = HttpServer::new(move || 
        App::new()
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(), 
                secret_key.clone()
            ))
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            //.route("/admin/dashboard", web::get().to(admin_dashboard))
            //.route("/admin/logout", web::post().to(log_out))
            //.route("/admin/password", web::get().to(change_password_form))
            //.route("/admin/password", web::post().to(change_password))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/health_check", web::get().to(health_check))
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_anonymous_users))
                    .route("/dashboard", web::get().to(admin_dashboard))
                    .route("/password", web::get().to(change_password_form))
                    .route("/password", web::post().to(change_password))
                    .route("/logout", web::post().to(log_out)),
            )
            .app_data(connection.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(Data::new(HmacSecret(hmac_secret.clone())))
    )
    .listen(listener)?
    .run();

    Ok(server)
}

#[derive(Clone)]
pub struct HmacSecret(pub SecretString);
