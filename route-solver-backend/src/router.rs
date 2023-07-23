//! The router module handles taking the flight itineraries, querying for pricing data and computing optimal travel routes.
//!
//! todo: Explain algorithm

use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{BTreeSet, BinaryHeap, HashMap},
    rc::Rc,
    slice,
};

use crate::flight_api::{LegQuery, PriceQuery, Quote, TestPriceApiQuery};
use actix_web::error::QueryPayloadError;
use route_solver_shared::queries::*;

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
    back_price: Option<f32>,
    price: Option<f32>,
    prev: Option<Rc<FlightNode>>,
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
        // let _graph_root = self.construct_graph(problem);

        todo!();
    }

    async fn expand_node<'a>(
        &mut self,
        src: Rc<FlightNode>,
        src_dr: &SingleDateRange,
        remaining_dests: slice::Iter<'a, Destination>,
        main_queue: &mut BinaryHeap<Rc<FlightNode>>,
    ) {
        for next_dest in remaining_dests {
            for possible_date in next_dest.dates.1.intersect(src_dr).iter() {
                // Create next nodes
                let flight = Flight {
                    src: src.flight.dest.clone(),
                    dest: next_dest.iata.clone(),
                    date: possible_date,
                };

                let price_query = self
                    .db
                    .get_price_for_flight::<TestPriceApiQuery>(&flight)
                    .await;

                let node = FlightNode {
                    flight,
                    price: Some(price_query),
                    back_price: Some(src.back_price.unwrap() + price_query),
                    prev: Some(Rc::clone(&src)),
                };

                // Insert into queue
                main_queue.push(Rc::new(node));
            }
        }
    }

    fn perform_graph_search(&mut self, problem: RouterProblem) -> FlightNode {
        // For a router problem, the anchors SRC and DEST are given at the front and back respectively of the Destination list, grab these
        let src = problem.dest_list[0].clone();
        let inter_dests_sl = &problem.dest_list[1..];

        let mut main_queue = BinaryHeap::<Rc<FlightNode>>::new();
        let init_dest_list = inter_dests_sl.to_vec();

        main_queue.push(Rc::new(FlightNode {
            flight: Flight {
                src: "".to_string(),
                dest: src.iata.clone(),
                date: Date::new(0, 0, 0),
            },
            back_price: Some(0.0),
            price: Some(0.0),
            prev: None,
        }));

        let final_node = loop {
            let top = main_queue.pop();
        };

        todo!();
    }
}

impl PartialEq<FlightNode> for FlightNode {
    fn eq(&self, other: &Self) -> bool {
        if let Some(s_p) = self.back_price {
            if let Some(o_p) = other.back_price {
                return s_p == o_p;
            }
        }

        false
    }
}

impl PartialOrd<FlightNode> for FlightNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if let Some(s_p) = self.back_price {
            if let Some(o_p) = other.back_price {
                if s_p < o_p {
                    return Some(Ordering::Greater);
                } else if s_p > o_p {
                    return Some(Ordering::Less);
                }

                return Some(Ordering::Equal);
            }
        }

        None
    }
}

impl Eq for FlightNode {}

impl Ord for FlightNode {
    fn cmp(&self, other: &Self) -> Ordering {
        if let Some(s_p) = self.back_price {
            if let Some(o_p) = other.back_price {
                if s_p < o_p {
                    return Ordering::Greater;
                } else if s_p > o_p {
                    return Ordering::Less;
                }

                return Ordering::Equal;
            }
        }

        panic!("Comparing empty price");
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
            date: SingleDateRange::FixedDate(flight.date.clone()),
        }]);

        leg_q.get_prices().await.unwrap()
    }

    async fn get_price_for_flight<Api: PriceQuery>(&mut self, flight: &Flight) -> f32 {
        let db_val = self.price_db.get(&flight);
        match db_val {
            Some(v) => *v,
            None => {
                // Need to query API
                let quote = self.query_api::<Api>(flight).await[0].min_price;
                self.price_db.insert(flight.clone(), quote);

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
