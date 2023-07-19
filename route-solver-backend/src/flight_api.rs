//! Flight API module to communicate to API.
//!
//! Handles communication with flight pricing API, right now we use the SkyScanner REST API.

use route_solver_shared::Queries::{Date, SingleDateRange};
use serde::{ser::SerializeStruct, Serialize};
use std::{collections::HashMap, time};

const SKYSCANNER_IND_PRICES_ENDPOINT: &str =
    "https://partners.api.skyscanner.net/apiservices/v3/flights/indicative/search";
const SKYSCANNER_PUB_API_KEY: &str = "sh428739766321522266746152871799";

pub struct LegQuery {
    start: String,
    end: String,
    date: SingleDateRange,
}

#[derive(Debug)]
pub enum QueryError {
    NoLegs,
    ResponseConversionErr(serde_json::Error, String),
    ReqwestErr(reqwest::Error),
    ResponseUnexpectedFormatErr(String),
    RateLimitExceeded,
    BadResponse(u16),
}

#[async_trait::async_trait]
pub trait PriceQuery {
    fn new(legs: Vec<LegQuery>) -> Self;
    async fn get_prices(&self) -> Result<Vec<Quote>, QueryError>;
}

pub struct SkyScannerApiQuery {
    curr_query: Query,
}

pub struct TestPriceApiQuery;

#[derive(Serialize)]
pub struct Query {
    market: String,
    locale: String,
    currency: String,
    queryLegs: Vec<LegQuery>,
    dateTimeGroupingType: String,
}

#[derive(Debug)]
pub struct Quote {
    pub min_price: f32,
    pub direct: bool,
}

impl Serialize for LegQuery {
    // Weirdly obnoxious query format, this just helps us serialize the LegQuery to the same format
    // as is expected by SkyScanner
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("", 3)?;
        state.serialize_field(
            "originPlace",
            &HashMap::from([("queryPlace", &HashMap::from([("iata", self.start.clone())]))]),
        )?;
        state.serialize_field(
            "destinationPlace",
            &HashMap::from([("queryPlace", &HashMap::from([("iata", self.end.clone())]))]),
        )?;
        match self.date {
            SingleDateRange::Anytime => state.serialize_field("anytime", &true),
            SingleDateRange::FixedDate(date) => state.serialize_field(
                "fixedDate",
                &HashMap::from([
                    ("year", date.year),
                    ("month", date.month),
                    ("day", date.day),
                ]),
            ),
            SingleDateRange::DateRange(date1, date2) => state.serialize_field(
                "dateRange",
                &HashMap::from([
                    (
                        "startDate",
                        HashMap::from([
                            ("year", date1.year),
                            ("month", date1.month),
                            ("day", date1.day),
                        ]),
                    ),
                    (
                        "endDate",
                        HashMap::from([
                            ("year", date2.year),
                            ("month", date2.month),
                            ("day", date2.day),
                        ]),
                    ),
                ]),
            ),
        }?;
        state.end()
    }
}

impl Query {
    fn new(market: String, currency: String, legs: Vec<LegQuery>) -> Query {
        Query {
            market,
            locale: "en-US".to_string(),
            currency,
            queryLegs: legs,
            dateTimeGroupingType: "DATE_TIME_GROUPING_TYPE_UNSPECIFIED".to_string(),
        }
    }
}

fn skyscanner_quote_to_price(val: &serde_json::Value) -> Result<Quote, QueryError> {
    use serde_json::Value::{Bool, Object, String};

    let Object(value) = val else {
        return Err(QueryError::ResponseUnexpectedFormatErr("Skyscanner quote has invalid format...".to_string()));
    };

    let price_str_val = &value["minPrice"]["amount"];
    let price = if let String(price_str) = price_str_val {
        price_str.parse::<f32>().map_err(|_| {
            QueryError::ResponseUnexpectedFormatErr(
                "Skyscanner quoted price cannot be converted to a number".to_string(),
            )
        })?
    } else {
        return Err(QueryError::ResponseUnexpectedFormatErr(
            "Price in quote is not a string type".to_string(),
        ));
    };

    let Bool(direct) = &value["isDirect"] else {
        return Err(QueryError::ResponseUnexpectedFormatErr("Is direct quote is not a bool type".to_string()));
    };

    Ok(Quote {
        min_price: price,
        direct: *direct,
    })
}

impl SkyScannerApiQuery {
    async fn get_indicative_prices_simplified(&self) -> Result<Vec<Quote>, QueryError> {
        use serde_json::Value::Object;

        let prices_resp = self.get_indicative_price().await?;
        let quotes = &prices_resp["content"]["results"]["quotes"];

        let Object(quotes_arr) = quotes else {
            return Err(QueryError::ResponseUnexpectedFormatErr("Skyscanner quotes section has an unexpected format".to_string()));
        };

        quotes_arr.values().map(skyscanner_quote_to_price).collect()
    }

    pub async fn get_indicative_price(&self) -> Result<serde_json::Value, QueryError> {
        let jquery = serde_json::to_string(&self.curr_query);
        let jquery = match jquery {
            Ok(s) => {
                format!("{{ \"query\": {} }}", s)
            }
            _ => {
                panic!("Error constructing query to indicative price");
            }
        };

        // Temporary, put this construction somewhere earlier and pass it through
        let client = reqwest::Client::new();
        let req = client
            .post(SKYSCANNER_IND_PRICES_ENDPOINT)
            .header("x-api-key", SKYSCANNER_PUB_API_KEY)
            .body(jquery)
            .send()
            .await
            .map_err(|e| QueryError::ReqwestErr(e))?
            .error_for_status()
            .map_err(|e| {
                if e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) {
                    QueryError::RateLimitExceeded
                } else {
                    let status_code = e
                        .status()
                        .expect("Fatal error gracefully handling status error")
                        .as_u16();
                    QueryError::BadResponse(status_code)
                }
            })?
            .text()
            .await
            .map_err(|e| QueryError::ReqwestErr(e))?;

        println!("Response from skyscanner: {}", req);
        let response_obj = serde_json::from_str(&req)
            .map_err(|e| QueryError::ResponseConversionErr(e, req.clone()))?;

        Ok(response_obj)
    }
}

#[async_trait::async_trait]
impl PriceQuery for SkyScannerApiQuery {
    fn new(legs: Vec<LegQuery>) -> Self {
        SkyScannerApiQuery {
            curr_query: Query::new("US".to_string(), "USD".to_string(), legs),
        }
    }

    async fn get_prices(&self) -> Result<Vec<Quote>, QueryError> {
        loop {
            let this_resp = self.get_indicative_prices_simplified().await;

            match this_resp {
                Err(QueryError::RateLimitExceeded) => {
                    println!("Flight API rate limit hit, sleeping");
                    std::thread::sleep(time::Duration::from_millis(250));
                }
                _ => break this_resp,
            };
        }
    }
}

#[async_trait::async_trait]
impl PriceQuery for TestPriceApiQuery {
    fn new(_: Vec<LegQuery>) -> Self {
        TestPriceApiQuery
    }

    async fn get_prices(&self) -> Result<Vec<Quote>, QueryError> {
        Ok(vec![
            Quote {
                min_price: 300.0,
                direct: true,
            },
            Quote {
                min_price: 400.0,
                direct: true,
            },
            Quote {
                min_price: 200.0,
                direct: true,
            },
        ])
    }
}

#[cfg(test)]
mod flight_api_tests {
    use route_solver_shared::Queries::Date;

    use crate::flight_api::LegQuery;
    use crate::flight_api::SingleDateRange;
    use crate::flight_api::{PriceQuery, SkyScannerApiQuery, TestPriceApiQuery};

    #[tokio::test]
    async fn test_sky_scanner_api_no_fail() {
        let api = SkyScannerApiQuery::new(vec![LegQuery {
            start: "JFK".to_string(),
            end: "YVR".to_string(),
            date: SingleDateRange::FixedDate(Date::new(10, 8, 2023)),
        }]);

        let quotes = api.get_prices().await;

        assert!(quotes.is_ok());

        if let Ok(res) = quotes {
            println!("{:?}", res[0]);
        }
    }

    #[tokio::test]
    async fn test_test_api_returns_static_values() {
        let api = TestPriceApiQuery;
        let quotes = api.get_prices().await;

        let Ok(quotes_unwrapped) = quotes else {
            assert!(false);
            panic!("What");
        };

        assert_eq!(quotes_unwrapped[0].min_price, 300.0);
        assert_eq!(quotes_unwrapped[1].min_price, 400.0);
        assert_eq!(quotes_unwrapped[2].min_price, 200.0);
    }
}
