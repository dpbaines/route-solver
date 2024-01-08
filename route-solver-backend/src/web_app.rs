//! Main web app module containing web routings to access API etc.

use actix_web::{get, post, web, HttpResponse, Responder, HttpRequest};
use serde::Deserialize;
use route_solver_shared::queries::{EchoQuery, RouteQuery};


#[derive(Deserialize)]
pub struct SingleHopPriceQuery {
    start_city: String,
    end_city: String,
}

#[get("/")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello")
}

// pub async fn index(rVeq: HttpRequest) -> Result<NamedFile> {
//     let path: PathBuf = req.match_info().query("filename").parse().unwrap();
//     Ok(NamedFile::open())
// }

#[post("/echo")]
pub async fn echo(json: web::Json<EchoQuery>) -> impl Responder {
    println!("Received: {0}", json.input);
    HttpResponse::Ok().body(format!("Received: {0}", json.input))
}

/// Endpoint for running route computation
#[post("/compute_route")]
pub async fn compute(json: web::Json<RouteQuery>) -> impl Responder {
    HttpResponse::Ok().body(format!(
        "Start city {0}, End City {1}, num_hops {2}",
        json.start_city,
        json.end_city,
        json.hops.len()
    ))
}

#[post("/get_price")]
pub async fn price(json: web::Json<SingleHopPriceQuery>) -> impl Responder {
    HttpResponse::Ok().body(format!(
        "Getting prices for flight: Start city {0}, End City {1}",
        json.start_city, json.end_city
    ))
}
