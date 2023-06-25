use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::collections::HashMap;

const SKYSCANNER_IND_PRICES_ENDPOINT: &str = "https://partners.api.skyscanner.net/apiservices/v3/flights/indicative/search";

fn new_indicative_query(market: String, locale: String, currency: String) {
    let q = r#"
        {
            "query": {
                "market": market,
                "locale": locale,
                "currency": currency,
                "queryLegs": [
                    
                ],
                "dateTimeGroupingType": "DATE_TIME_GROUPING_TYPE_UNSPECIFIED"
            }
        }
    "#; 
}

// pub fn get_indicative_price(src: String, dest: String) -> f32 {
//     let client = Client::new();
//     let uri = "http://httpbin.org/ip".parse()?;
//     let resp = client.get(uri).await?;
//
//     let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
//     let st = String::from_utf8(body_bytes.to_vec()).unwrap();
//     println!("Response: {}", st);
// }

