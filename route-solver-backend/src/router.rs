//! The router module handles taking the flight itineraries, querying for pricing data and computing optimal travel routes.
//!
//! todo: Explain algorithm

use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    rc::Rc,
};

use crate::flight_api::{LegQuery, PriceQuery, Quote};
use route_solver_shared::Queries::*;

struct RouterDb {
    price_db: HashMap<Flight, f32>,
}

struct RouterProblem {
    total_date_range: DateRange,
    dest_list: Vec<Destination>,
}

/// Main router class, maintains a database of already seen prices.
struct Router {
    db: RouterDb,
}

/// Graph node for main flights graph. The flights graph represents all possible flight/date combinations given the route problem.
///
/// Each node contains a [Flight](route_solver_shared::Queries::Flight), a price, and an child list. Price is lazy loaded to not kill the API.
struct FlightNode {
    flight: Flight,
    price: Option<f32>,
    childs: Vec<Rc<RefCell<FlightNode>>>,
}

impl Router {
    fn new() -> Router {
        Router {
            db: RouterDb::new(),
        }
    }

    /// Main solver routine, takes in problem and outputs route.
    ///
    /// The algorithm performs the following general steps to create the route
    /// 1. Construct a graph of all possible ```Flight```s between the anchor SRC and anchor DEST
    ///     a. A ```Flight``` represents a src/dest with a date of travel
    ///     b. Each node on the graph represents a flight with a cost of that flight (lazy calculated)
    /// 2. Djikstra search from SRC to DEST anchor
    fn calc(&mut self, problem: RouterProblem) -> Vec<FlightPrice> {
        let _graph_root = self.construct_graph(problem);

        todo!();
    }

    /// Helper function to graph constructor, recursively generates the DAG ensuring that constraints are met.
    fn construct_graph_helper(
        &mut self,
        dest: &Destination,
        remaining_dests: &BTreeSet<Destination>,
    ) -> Option<FlightNode> {
        todo!();
    }

    fn construct_graph(&mut self, problem: RouterProblem) -> FlightNode {
        // For a router problem, the anchors SRC and DEST are given at the front and back respectively of the Destination list, grab these
        let src = problem.dest_list[0].clone();
        let dest = problem.dest_list.last().cloned();
        let inter_dests_sl = &problem.dest_list[1..(problem.dest_list.len() - 1)];

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
