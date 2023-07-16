use std::{collections::HashMap, time};

use crate::flight_api::{LegQuery, PriceQuery, QueryDateRange, Quote};
use route_solver_shared::Queries::*;

struct RouterDb {
    price_db: HashMap<Flight, f32>,
}

struct RouterProblem {
    total_date_range: DateRange,
    dest_list: Vec<Destination>,
}

struct Router {
    db: RouterDb,
}

impl Router {
    fn new() -> Router {
        Router {
            db: RouterDb::new(),
        }
    }

    fn calc(&self, problem: RouterProblem) -> Vec<FlightPrice> {
        vec![
            FlightPrice {
                flight: Flight {
                    src: "YVR".to_string(),
                    dest: "HND".to_string(),
                    date: Date::new(6, 6, 2023),
                },
                price: 300.0,
            },
            FlightPrice {
                flight: Flight {
                    src: "HND".to_string(),
                    dest: "INC".to_string(),
                    date: Date::new(16, 6, 2023),
                },
                price: 200.0,
            },
            FlightPrice {
                flight: Flight {
                    src: "INC".to_string(),
                    dest: "BKK".to_string(),
                    date: Date::new(20, 6, 2023),
                },
                price: 150.0,
            },
        ]
    }
}

impl RouterDb {
    fn new() -> RouterDb {
        RouterDb {
            price_db: HashMap::new(),
        }
    }

    async fn query_api<Api: PriceQuery>(&self, flight: &Flight) -> Vec<Quote> {
        let leg_q = Api::new(vec![LegQuery {
            start: flight.src.clone(),
            end: flight.dest.clone(),
            date: QueryDateRange::FixedDate(flight.date.clone()),
        }]);

        let quotes_res = leg_q.get_prices().await.unwrap();

        quotes_res
    }

    async fn get_price_for_flight<Api: PriceQuery>(&mut self, flight: Flight) -> f32 {
        let db_val = self.price_db.get(&flight);
        match db_val {
            Some(v) => *v,
            None => {
                // Need to query API
                let quote = self.query_api::<Api>(&flight).await[0].min_price;
                self.price_db.insert(flight, quote);

                quote
            }
        }
    }
}

#[cfg(test)]
mod router_tests {
    #[test]
    fn test_flight_db() {}
}
