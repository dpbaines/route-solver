//! The router module handles taking the flight itineraries, querying for pricing data and computing optimal travel routes.
//!
//! todo: Explain algorithm

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    rc::Rc,
};

use crate::flight_api::PriceQuery;
use route_solver_shared::queries::*;

struct RouterProblem {
    total_date_range: DateRange,
    dest_list: Vec<Destination>,
}

/// Main router class, maintains a database of already seen prices.
struct Router<Api: PriceQuery> {
    api: Api,
}

/// Graph node for main flights graph. The flights graph represents all possible flight/date combinations given the route problem.
///
/// Each node contains a [Flight](route_solver_shared::Queries::Flight), a price, and an child list. Price is lazy loaded to not kill the API.
#[derive(Debug)]
struct FlightNode {
    flight: Flight,
    back_price: Option<f32>,
    price: Option<f32>,
    prev: Option<Rc<FlightNode>>,
}

impl<Api: PriceQuery> Router<Api> {
    fn new() -> Router<Api> {
        Router { api: Api::new() }
    }

    /// Main solver routine, takes in problem and outputs route.
    ///
    /// The algorithm performs the following general steps to create the route
    /// 1. Construct a graph of all possible ```Flight```s between the anchor SRC and anchor DEST
    ///     a. A ```Flight``` represents a src/dest with a date of travel
    ///     b. Each node on the graph represents a flight with a cost of that flight (lazy calculated)
    /// 2. Djikstra search from SRC to DEST anchor
    async fn calc(&mut self, problem: RouterProblem) -> Vec<FlightPrice> {
        let problem_res = self.perform_graph_search(problem).await;

        // For now panic if flight not possible
        let problem_res_unwrap = problem_res.unwrap();
        problem_res_unwrap
            .iter()
            .map(|f| FlightPrice {
                flight: f.flight.clone(),
                price: f.price.unwrap(),
            })
            .collect()
    }

    fn backtrace_helper(&mut self, curr_node: Rc<FlightNode>, output: &mut Vec<Rc<FlightNode>>) {
        if let Some(prev) = &curr_node.prev {
            self.backtrace_helper(Rc::clone(prev), output);
        }

        output.push(curr_node);
    }

    fn backtrace_node(&mut self, final_node: Rc<FlightNode>) -> Vec<Rc<FlightNode>> {
        let mut output_vec = Vec::<Rc<FlightNode>>::new();
        self.backtrace_helper(final_node, &mut output_vec);

        output_vec
    }

    async fn expand_node(
        &mut self,
        src: Rc<FlightNode>,
        src_dr: &SingleDateRange,
        remaining_dests: Vec<Destination>,
        main_queue: &mut BinaryHeap<Rc<FlightNode>>,
    ) {
        for next_dest in remaining_dests.iter() {
            for possible_date in next_dest.dates.0.intersect(src_dr).iter() {
                // Create next nodes
                let flight = Flight {
                    src: src.flight.dest.clone(),
                    dest: next_dest.iata.clone(),
                    date: possible_date,
                };

                let price_query = self.api.get_price(flight.clone()).await.unwrap().min_price;

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

    async fn perform_graph_search(
        &mut self,
        problem: RouterProblem,
    ) -> Result<Vec<Rc<FlightNode>>, String> {
        // For a router problem, the anchors SRC and DEST are given at the front and back respectively of the Destination list, grab these
        let src = problem.dest_list[0].clone();
        let inter_dests_sl = &problem.dest_list[1..(problem.dest_list.len() - 2)];
        let final_dest = &problem.dest_list[problem.dest_list.len() - 1];

        let mut main_queue = BinaryHeap::<Rc<FlightNode>>::new();
        let init_dest_list = inter_dests_sl.to_vec();

        let dest_date_map = HashMap::<String, DateRange>::from_iter(
            problem
                .dest_list
                .iter()
                .map(|e| (e.iata.clone(), e.dates.clone())),
        );

        // TODO: Generalize flight data to be able to include more or less metadata depending on the API
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

        let final_node: Rc<FlightNode> = loop {
            let top = main_queue.pop();

            if let None = top {
                break None;
            }

            let top_n = top.unwrap();

            if top_n.flight.dest == final_dest.iata {
                break Some(top_n);
            }

            let src_dr = &dest_date_map.get(&top_n.flight.dest).unwrap().1;

            let filter_pred = |e: &Destination| -> Option<Destination> {
                // TODO: The way we do this makes having duplicate city entries in itinerary unsupported...
                let mut curr_node = Rc::clone(&top_n);

                if curr_node.flight.dest == e.iata {
                    return None;
                }
                while let Some(prev) = &curr_node.prev {
                    if prev.flight.dest == e.iata {
                        return None;
                    }

                    curr_node = Rc::clone(&prev);
                }

                Some(e.clone())
            };

            let mut dest_list: Vec<Destination> =
                init_dest_list.iter().filter_map(filter_pred).collect();
            if dest_list.len() == 0 {
                dest_list.push(final_dest.clone());
            }

            // Can afford to linear search path and filter nodes that exist, path's aren't going to be long (hopefully)
            self.expand_node(Rc::clone(&top_n), src_dr, dest_list, &mut main_queue)
                .await;
        }
        .ok_or("Itinerary cannot solve, adjust parameters".to_string())?;

        let list_flights = self.backtrace_node(final_node);

        Ok(list_flights)
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

#[cfg(test)]
mod router_tests {
    use std::{collections::BinaryHeap, rc::Rc};

    use route_solver_shared::queries::{Date, DateRange, Destination, Flight, SingleDateRange};

    use crate::flight_api::TestPriceApiQuery;

    use super::{FlightNode, Router};

    #[tokio::test]
    async fn test_heap_expand() {
        let mut router = Router::<TestPriceApiQuery>::new();
        let node_to_expand = Rc::new(FlightNode {
            flight: Flight {
                src: "YYZ".to_string(),
                dest: "YVR".to_string(),
                date: Date::new(1, 2, 2023),
            },
            back_price: Some(0.0),
            price: Some(250.0),
            prev: None,
        });

        let test_dest_vec = vec![
            Destination {
                iata: "YYC".to_string(),
                dates: DateRange(
                    SingleDateRange::FixedDate(Date::new(3, 2, 2023)),
                    SingleDateRange::None,
                ),
            },
            Destination {
                iata: "SEA".to_string(),
                dates: DateRange(
                    SingleDateRange::DateRange(Date::new(2, 2, 2023), Date::new(6, 2, 2023)),
                    SingleDateRange::None,
                ),
            },
            Destination {
                iata: "YYZ".to_string(),
                dates: DateRange(
                    SingleDateRange::FixedDate(Date::new(4, 2, 2023)),
                    SingleDateRange::None,
                ),
            },
        ];

        let node_date_range =
            SingleDateRange::DateRange(Date::new(2, 2, 2023), Date::new(4, 2, 2023));

        let mut main_queue = BinaryHeap::<Rc<FlightNode>>::new();

        router
            .expand_node(
                node_to_expand,
                &node_date_range,
                test_dest_vec,
                &mut main_queue,
            )
            .await;

        let heap_vec = main_queue.into_vec();
        println!("{:?}", heap_vec);

        assert!(heap_vec
            .iter()
            .find(|e| {
                e.flight
                    == Flight {
                        src: "YVR".to_string(),
                        dest: "YYC".to_string(),
                        date: Date::new(3, 2, 2023),
                    }
            })
            .is_some());

        assert!(heap_vec
            .iter()
            .find(|e| {
                e.flight
                    == Flight {
                        src: "YVR".to_string(),
                        dest: "SEA".to_string(),
                        date: Date::new(2, 2, 2023),
                    }
            })
            .is_some());
        assert!(heap_vec
            .iter()
            .find(|e| {
                e.flight
                    == Flight {
                        src: "YVR".to_string(),
                        dest: "SEA".to_string(),
                        date: Date::new(3, 2, 2023),
                    }
            })
            .is_some());
        assert!(heap_vec
            .iter()
            .find(|e| {
                e.flight
                    == Flight {
                        src: "YVR".to_string(),
                        dest: "SEA".to_string(),
                        date: Date::new(4, 2, 2023),
                    }
            })
            .is_some());
        assert!(heap_vec
            .iter()
            .find(|e| {
                e.flight
                    == Flight {
                        src: "YVR".to_string(),
                        dest: "YYZ".to_string(),
                        date: Date::new(4, 2, 2023),
                    }
            })
            .is_some());

        assert_eq!(heap_vec.len(), 5);
    }
}
