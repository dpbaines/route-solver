use actix_web::{get, post, web, HttpResponse, Responder};
use serde::Deserialize;

// #[function_component(App)]
// pub fn app() -> Html {
//     html! {
//         <main>
//             <h1>{ "Hello World!" }</h1>
//             <span class="subtitle">{ "from Yew with " }<i class="heart" /></span>
//         </main>
//     }
// }

#[derive(Deserialize)]
pub struct RouteQuery {
    start_city: String,
    end_city: String,
    hops: Vec<String>,
}

#[get("/")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Frontend TBD")
}

#[post("/compute_route")]
pub async fn compute(json: web::Json<RouteQuery>) -> impl Responder {
    HttpResponse::Ok().body(format!("Start city {0}, End City {1}, num_hops {2}", json.start_city, json.end_city, json.hops.len()))
}

pub async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}
