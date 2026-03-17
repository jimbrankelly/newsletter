use actix_web::{web, App, HttpServer};
//use actix_web::middleware::Logger;
use tracing_actix_web::TracingLogger;
use actix_web::dev::Server;
use std::net::TcpListener;
use sqlx::{PgPool, postgres::PgPoolOptions,};

use crate::{
  configuration::{Settings, DatabaseSettings, },
  email_client::EmailClient,
  routes::{health_check, subscribe},
};

// A new type to hold the newly built server and its port 
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
  pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
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
      let server = run(listener, connection_pool, email_client)?;

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
