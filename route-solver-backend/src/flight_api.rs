use std::collections::HashMap;
use serde::{Serialize, ser::SerializeStruct};
use serde_json::Map;

const SKYSCANNER_IND_PRICES_ENDPOINT: &str = "https://partners.api.skyscanner.net/apiservices/v3/flights/indicative/search";
const SKYSCANNER_PUB_API_KEY: &str = "sh428739766321522266746152871799";

/// Date format, D/M/Y
type Date = (i16, i16, i16);

pub enum QueryDateRange {
    Anytime,
    FixedDate (Date),
    DateRange (Date, Date),
}

pub struct LegQuery {
    start: String,
    end: String,
    date: QueryDateRange,
}

#[derive(Debug)]
pub enum QueryError {
    ResponseConversionErr(serde_json::Error, String),
    ReqwestErr(reqwest::Error),
    ResponseUnexpectedFormatErr(String),
}

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
    min_price: f32,
    direct: bool,
}

impl Serialize for LegQuery {
    // Weirdly obnoxious query format, this just helps us serialize the LegQuery to the same format
    // as is expected by SkyScanner
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("", 3)?;
        state.serialize_field("originPlace", &HashMap::from([
                ("queryPlace", &HashMap::from([("iata", self.start.clone())]))
        ]))?;
        state.serialize_field("destinationPlace", &HashMap::from([
                ("queryPlace", &HashMap::from([("iata", self.end.clone())]))
        ]))?;
        match self.date {
            QueryDateRange::Anytime => state.serialize_field("anytime", &true),
            QueryDateRange::FixedDate ((d, m, y)) => state.serialize_field("fixedDate", &HashMap::from([
                ("year", y),
                ("month", m),
                ("day", d),
            ])),
            QueryDateRange::DateRange ((d1, m1, y1), (d2, m2, y2)) => state.serialize_field("dateRange", &HashMap::from([
                ("startDate", HashMap::from([
                    ("year", y1),
                    ("month", m1),
                    ("day", d1),
                ])),
                ("endDate", HashMap::from([
                    ("year", y2),
                    ("month", m2),
                    ("day", d2),
                ])),
            ])),
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
            dateTimeGroupingType: "DATE_TIME_GROUPING_TYPE_UNSPECIFIED".to_string()
        }
    }
}

pub fn skyscanner_quote_to_price(val: &serde_json::Value) -> Result<Quote, QueryError> {
    use serde_json::Value::{Bool, String, Object};

    let Object(value) = val else {
        return Err(QueryError::ResponseUnexpectedFormatErr("Skyscanner quote has invalid format...".to_string()));
    };

    let price_str_val = &value["minPrice"]["amount"];
    let price = if let String(price_str) = price_str_val {
        price_str.parse::<f32>()
            .map_err(|_| QueryError::ResponseUnexpectedFormatErr("Skyscanner quoted price cannot be converted to a number".to_string()))?
    } else {
        return Err(QueryError::ResponseUnexpectedFormatErr("Price in quote is not a string type".to_string()));
    };

    let Bool(direct) = &value["isDirect"] else {
        return Err(QueryError::ResponseUnexpectedFormatErr("Is direct quote is not a bool type".to_string()));
    };

    Ok(Quote {
        min_price: price,
        direct: *direct,
    })
}

pub async fn get_indicative_prices_simplified(legs: Vec<LegQuery>) -> Result<Vec<Quote>, QueryError> {
    use serde_json::Value::Object;

    let prices_resp = get_indicative_price(legs).await?;
    let quotes = &prices_resp["content"]["results"]["quotes"];

    let Object(quotes_arr) = quotes else {
        return Err(QueryError::ResponseUnexpectedFormatErr("Skyscanner quotes section has an unexpected format".to_string()));
    };

    quotes_arr.values().map(skyscanner_quote_to_price).collect()
}

pub async fn get_indicative_price(legs: Vec<LegQuery>) -> Result<serde_json::Value, QueryError> {
    let query = Query::new("US".to_string(), "USD".to_string(), legs);

    let jquery = serde_json::to_string(&query);
    let jquery = match jquery {
        Ok(s) => {
            format!("{{ \"query\": {} }}", s)
        },
        _ => {
            panic!("Error constructing query to indicative price");
        },
    };

    // Temporary, put this construction somewhere earlier and pass it through
    let client = reqwest::Client::new();
    let req = client.post(SKYSCANNER_IND_PRICES_ENDPOINT)
        .header("x-api-key", SKYSCANNER_PUB_API_KEY)
        .body(jquery)
        .send()
        .await
        .map_err(|e| QueryError::ReqwestErr(e))?
        .text()
        .await
        .map_err(|e| QueryError::ReqwestErr(e))?;

    println!("Response from skyscanner: {}", req);
    let response_obj = serde_json::from_str(&req).map_err(|e| QueryError::ResponseConversionErr(e, req.clone()))?;

    Ok(response_obj)
}

#[cfg(test)]
mod flight_api_tests {
    use crate::flight_api::LegQuery;
    use crate::flight_api::QueryDateRange;

    #[tokio::test]
    async fn test_get_ind_price_basic() {
        let _ = super::get_indicative_price(vec![LegQuery {
            start: "JFK".to_string(),
            end: "YVR".to_string(),
            date: QueryDateRange::FixedDate((10, 8, 2023)),
        }]).await;

        let quotes = super::get_indicative_prices_simplified(vec![LegQuery {
            start: "JFK".to_string(),
            end: "YVR".to_string(),
            date: QueryDateRange::FixedDate((10, 8, 2023)),
        }]).await;

        assert!(quotes.is_ok());

        if let Ok(res) = quotes {
            println!("{:?}", res[0]);
        }
    }
}

