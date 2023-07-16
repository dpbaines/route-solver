pub mod flight_api;
pub mod router;
pub mod web_app;

use actix_web::{App, HttpServer};

// use hyper::{Client, Result};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(web_app::hello).service(web_app::compute))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

// #[allow(dead_code)]
// fn temp() -> Result<(), _> {
//     println!("Hello, world!");
//     let client = Client::new();
//     let uri = "http://httpbin.org/ip".parse()?;
//     let resp = client.get(uri).await?;
//
//     let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
//     let st = String::from_utf8(body_bytes.to_vec()).unwrap();
//     println!("Response: {}", st);
//
//     Ok(())
// }
