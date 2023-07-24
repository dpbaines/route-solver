//! Flight API module to communicate to API.
//!
//! Handles communication with flight pricing API, right now we use the SkyScanner REST API.

use route_solver_shared::queries::{Date, Flight, SingleDateRange};
use serde::{ser::SerializeStruct, Serialize};
use std::{collections::HashMap, time};
use thiserror::Error;

const SKYSCANNER_IND_PRICES_ENDPOINT: &str =
    "https://partners.api.skyscanner.net/apiservices/v3/flights/indicative/search";
const SKYSCANNER_PUB_API_KEY: &str = "sh428739766321522266746152871799";

#[derive(Clone)]
pub struct LegQuery {
    pub start: String,
    pub end: String,
    pub date: SingleDateRange,
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("No legs were provided.")]
    NoLegs,
    #[error("Error deserializing JSON response from API.")]
    ResponseConversionErr(serde_json::Error, String),
    #[error("Error from reqwest.")]
    ReqwestErr(reqwest::Error),
    #[error("Response format is unexpected, cannot deserialize.")]
    ResponseUnexpectedFormatErr(String),
    #[error("Rate limit for API exceeded")]
    RateLimitExceeded,
    #[error("Bad response from API.")]
    BadResponse(u16),
    #[error("[Test only] legs don't exist")]
    NonExistentLeg,
}

#[async_trait::async_trait]
pub trait PriceQuery {
    fn new() -> Self;
    async fn get_price(&mut self, flight: Flight) -> Result<Quote, QueryError>;
}

pub struct SkyScannerApiQuery {
    db: HashMap<Flight, Quote>,
}

pub struct TestPriceApiQuery {
    data: HashMap<Flight, f32>,
}

#[derive(Serialize)]
pub struct Query {
    market: String,
    locale: String,
    currency: String,
    #[serde(rename = "queryLegs")]
    query_legs: Vec<LegQuery>,
    #[serde(rename = "dateTimeGroupingType")]
    date_time_grouping_type: String,
}

#[derive(Debug, Clone, Copy)]
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
        match &self.date {
            SingleDateRange::None => {
                panic!("Should not be sending a single date range none type to sky scanner")
            }
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
            query_legs: legs,
            date_time_grouping_type: "DATE_TIME_GROUPING_TYPE_UNSPECIFIED".to_string(),
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
    async fn get_indicative_prices_simplified_retry(
        &self,
        legs: Vec<LegQuery>,
    ) -> Result<Vec<Quote>, QueryError> {
        loop {
            let this_resp = self.get_indicative_prices_simplified(legs.clone()).await;

            match this_resp {
                Err(QueryError::RateLimitExceeded) => {
                    println!("Flight API rate limit hit, sleeping");
                    std::thread::sleep(time::Duration::from_millis(250));
                }
                _ => break this_resp,
            };
        }
    }

    async fn get_indicative_prices_simplified(
        &self,
        legs: Vec<LegQuery>,
    ) -> Result<Vec<Quote>, QueryError> {
        use serde_json::Value::Object;

        let prices_resp = self.get_indicative_price(legs).await?;
        let quotes = &prices_resp["content"]["results"]["quotes"];

        let Object(quotes_arr) = quotes else {
            return Err(QueryError::ResponseUnexpectedFormatErr("Skyscanner quotes section has an unexpected format".to_string()));
        };

        quotes_arr.values().map(skyscanner_quote_to_price).collect()
    }

    pub async fn get_indicative_price(
        &self,
        legs: Vec<LegQuery>,
    ) -> Result<serde_json::Value, QueryError> {
        // TODO: Query options configurable
        let jquery = serde_json::to_string(&Query::new("US".to_string(), "USD".to_string(), legs));
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
    fn new() -> Self {
        SkyScannerApiQuery { db: HashMap::new() }
    }

    async fn get_price(&mut self, flight: Flight) -> Result<Quote, QueryError> {
        let leg_q = vec![LegQuery {
            start: flight.src.clone(),
            end: flight.dest.clone(),
            date: SingleDateRange::FixedDate(flight.date.clone()),
        }];

        let db_val = self.db.get(&flight);
        match db_val {
            Some(v) => Ok(v.clone()),
            None => {
                // Need to query API
                let quote = self.get_indicative_prices_simplified_retry(leg_q).await?[0];
                self.db.insert(flight.clone(), quote);

                Ok(quote)
            }
        }
    }
}

#[async_trait::async_trait]
impl PriceQuery for TestPriceApiQuery {
    fn new() -> Self {
        // Load CSV, populate map
        let mut rdr = csv::Reader::from_path("test/MockPricingAirline.csv").unwrap();
        let mut data = HashMap::new();

        // For the input CSV, rows go YYZ, YVR, YYC, SEA
        let mut row_count: i32 = 0;

        let translate = |i| match i {
            0 => "YYZ",
            1 => "YVR",
            2 => "YYC",
            3 => "SEA",
            _ => "WTF",
        };

        let mut day_count = 1;

        for row in rdr.records() {
            let record = row.unwrap();
            for src_idx in 0..4 {
                let insert = Flight {
                    src: translate(src_idx).to_string(),
                    dest: translate(row_count).to_string(),
                    date: Date::new(day_count, 2, 2023),
                };

                data.insert(insert, record[src_idx as usize].parse::<f32>().unwrap());
            }

            row_count = (row_count + 1) % 4;
            if row_count == 0 {
                day_count += 1;
            }
        }

        TestPriceApiQuery { data }
    }

    async fn get_price(&mut self, flight: Flight) -> Result<Quote, QueryError> {
        let val = self.data.get(&flight).ok_or(QueryError::NonExistentLeg)?;
        Ok(Quote {
            min_price: *val,
            direct: false,
        })
    }
}

#[cfg(test)]
mod flight_api_tests {
    use crate::flight_api::{PriceQuery, SkyScannerApiQuery, TestPriceApiQuery};
    use route_solver_shared::queries::Date;
    use route_solver_shared::queries::Flight;

    #[tokio::test]
    async fn test_sky_scanner_api_no_fail() {
        let mut api = SkyScannerApiQuery::new();

        let quote = api
            .get_price(Flight {
                src: "JFK".to_string(),
                dest: "YVR".to_string(),
                date: Date::new(10, 8, 2023),
            })
            .await
            .unwrap();

        println!("{:?}", quote.min_price);
    }

    #[tokio::test]
    async fn test_test_api_returns_basic_values() {
        let mut api = TestPriceApiQuery::new();
        let quotes = api
            .get_price(Flight {
                src: "YYZ".to_string(),
                dest: "YYC".to_string(),
                date: Date::new(1, 2, 2023),
            })
            .await
            .unwrap();

        assert_eq!(quotes.min_price, 300.0);
    }
}
