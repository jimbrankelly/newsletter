use actix_web::{web, App, HttpServer};
//use actix_web::middleware::Logger;
use tracing_actix_web::TracingLogger;
use actix_web::dev::Server;
use std::net::TcpListener;
use sqlx::PgPool;

use crate::routes::{health_check, subscribe};

pub fn run(
    listener: TcpListener,
    // New parameter!
    connection_pool: PgPool
) -> Result<Server, std::io::Error> {
    // Wrap the connection in a smart pointer
    let connection = web::Data::new(connection_pool);

    let server = HttpServer::new(move || 
        App::new()
            //.wrap(Logger::default())
            // Instead of `Logger::default`
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            // Register the connection as part of the application state
            .app_data(connection.clone())
    )
    .listen(listener)?
    .run();

    Ok(server)
}
