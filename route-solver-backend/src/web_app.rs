//! Main web app module containing web routings to access API etc.

use actix_web::{get, post, web, HttpResponse, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RouteQuery {
    start_city: String,
    end_city: String,
    hops: Vec<String>,
}

#[derive(Deserialize)]
pub struct SingleHopPriceQuery {
    start_city: String,
    end_city: String,
}

#[get("/")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Frontend TBD")
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
