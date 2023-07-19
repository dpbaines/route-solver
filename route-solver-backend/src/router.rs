use std::collections::HashMap;

use crate::flight_api::{LegQuery, PriceQuery, Quote};
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
        todo!();
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
            date: flight.date.clone(),
        }]);

        leg_q.get_prices().await.unwrap()
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
