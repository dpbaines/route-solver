//! Backend crate for pathfinder web app.
//!
//! Uses actix to serve the backend functionality, importantly taking in user travel itineraries and optimizing.

pub mod flight_api;
pub mod router;
pub mod web_app;

use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(web_app::hello).service(web_app::compute))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
