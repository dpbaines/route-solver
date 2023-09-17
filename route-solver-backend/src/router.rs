//! The router module handles taking the flight itineraries, querying for pricing data and computing optimal travel routes.
//!
//! todo: Explain algorithm

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    fmt,
    rc::Rc,
};

use crate::flight_api::PriceQuery;
use route_solver_shared::queries::*;

struct RouterProblem {
    dest_list: Vec<Destination>,
}

/// Router Stats
struct RouterStats {
    api_calls: u16,
    enabled: bool,
}

/// Main router class, maintains a database of already seen prices.
struct Router<Api: PriceQuery> {
    api: Api,
    stats: RouterStats,
}

/// Wrapper for the result of the solve
struct RouterResult {
    result: Vec<FlightPrice>,
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
    dest_ref: Destination,
}

impl RouterStats {
    fn new() -> RouterStats {
        RouterStats {
            api_calls: 0,
            enabled: true,
        }
    }

    fn record_call(&mut self) {
        if self.enabled {
            self.api_calls += 1;
        }
    }
}

impl fmt::Display for RouterStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Api calls: {}", self.api_calls)
    }
}

impl RouterResult {
    fn total_price(&self) -> f32 {
        self.result.iter().fold(0.0, |acc, f| acc + f.price)
    }
}

impl fmt::Display for RouterResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = "".to_string();
        for flight in &self.result {
            let curr_val = format!(
                "{} : {} -> {} : ${}, ",
                flight.flight.date, flight.flight.src, flight.flight.dest, flight.price
            );
            res += &curr_val;
        }

        write!(f, "{}", res)
    }
}

impl<Api: PriceQuery> Router<Api> {
    fn new() -> Router<Api> {
        Router {
            api: Api::new(),
            stats: RouterStats::new(),
        }
    }

    /// Main solver routine, takes in problem and outputs route.
    ///
    /// The algorithm performs the following general steps to create the route
    /// 1. Construct a graph of all possible ```Flight```s between the anchor SRC and anchor DEST
    ///     a. A ```Flight``` represents a src/dest with a date of travel
    ///     b. Each node on the graph represents a flight with a cost of that flight (lazy calculated)
    /// 2. Djikstra search from SRC to DEST anchor
    async fn calc(&mut self, problem: RouterProblem) -> RouterResult {
        let problem_res = self.perform_graph_search(problem).await;

        // For now panic if flight not possible
        let problem_res_unwrap = problem_res.unwrap();
        RouterResult {
            result: problem_res_unwrap
                .iter()
                .skip(1) // First node is a dummy for seeding heap expansion
                .map(|f| FlightPrice {
                    flight: f.flight.clone(),
                    price: f.price.unwrap(),
                })
                .collect(),
        }
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
        remaining_dests: Vec<Destination>,
        main_queue: &mut BinaryHeap<Rc<FlightNode>>,
    ) {
        for next_dest in remaining_dests.iter() {
            for possible_date in next_dest.dates.0.intersect(&src.dest_ref.dates.1).iter() {
                // Create next nodes
                let flight = Flight {
                    src: src.flight.dest.clone(),
                    dest: next_dest.iata.clone(),
                    date: possible_date,
                };

                let price_query = self.api.get_price(flight.clone()).await.unwrap().min_price;
                self.stats.record_call();

                let node = FlightNode {
                    flight,
                    price: Some(price_query),
                    back_price: Some(src.back_price.unwrap() + price_query),
                    prev: Some(Rc::clone(&src)),
                    dest_ref: next_dest.clone(),
                };

                // Insert into queue
                main_queue.push(Rc::new(node));
            }
        }
    }

    fn fill_dest_list(
        &self,
        curr_node: Rc<FlightNode>,
        final_dest: &Destination,
        init_dest_list: &Vec<Destination>,
    ) -> Vec<Destination> {
        let filter_pred = |e: &Destination| -> Option<Destination> {
            // TODO: The way we do this makes having duplicate city entries in itinerary unsupported...
            if curr_node.flight.dest == e.iata {
                return None;
            }

            let mut temp_curr_node = &curr_node;
            while let Some(prev) = &temp_curr_node.prev {
                if prev.flight.dest == e.iata {
                    return None;
                }

                temp_curr_node = &prev;
            }

            Some(e.clone())
        };

        let mut dest_list: Vec<Destination> =
            init_dest_list.iter().filter_map(filter_pred).collect();
        if dest_list.len() == 0 {
            dest_list.push(final_dest.clone());
        }

        dest_list
    }

    async fn perform_graph_search(
        &mut self,
        problem: RouterProblem,
    ) -> Result<Vec<Rc<FlightNode>>, String> {
        // For a router problem, the anchors SRC and DEST are given at the front and back respectively of the Destination list, grab these
        let src = problem.dest_list[0].clone();
        let inter_dests_sl = &problem.dest_list[1..(problem.dest_list.len() - 1)];
        let final_dest = &problem.dest_list[problem.dest_list.len() - 1];

        let mut main_queue = BinaryHeap::<Rc<FlightNode>>::new();
        let init_dest_list = inter_dests_sl.to_vec();

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
            dest_ref: src.clone(),
        }));

        let final_node: Rc<FlightNode> = loop {
            let top = main_queue.pop();

            if let None = top {
                break None;
            }

            let top_n = top.unwrap();

            if top_n.flight.dest == final_dest.iata && top_n.prev.is_some() {
                break Some(top_n);
            }

            // Can afford to linear search path and filter nodes that exist, path's aren't going to be long (hopefully)
            let dest_list = self.fill_dest_list(Rc::clone(&top_n), &final_dest, &init_dest_list);

            self.expand_node(Rc::clone(&top_n), dest_list, &mut main_queue)
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

    use crate::{flight_api::TestPriceApiQuery, router::RouterProblem};

    use super::{FlightNode, Router};

    #[tokio::test]
    async fn test_heap_expand() {
        let node_date_range =
            SingleDateRange::DateRange(Date::new(2, 2, 2023), Date::new(4, 2, 2023));

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
            dest_ref: Destination {
                iata: "YYZ".to_string(),
                dates: DateRange(SingleDateRange::None, node_date_range),
            },
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

        let mut main_queue = BinaryHeap::<Rc<FlightNode>>::new();

        router
            .expand_node(node_to_expand, test_dest_vec, &mut main_queue)
            .await;

        let heap_vec = main_queue.into_vec();

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

    #[test]
    fn test_dest_list_fill() {
        let mut router = Router::<TestPriceApiQuery>::new();
        let init_dest_list = vec![
            Destination {
                // Source
                iata: "YYZ".to_string(),
                dates: DateRange(
                    SingleDateRange::None,
                    SingleDateRange::DateRange(Date::new(1, 2, 2023), Date::new(3, 2, 2023)),
                ),
            },
            Destination {
                iata: "YVR".to_string(),
                dates: DateRange(
                    SingleDateRange::DateRange(Date::new(2, 2, 2023), Date::new(4, 2, 2023)),
                    SingleDateRange::DateRange(Date::new(4, 2, 2023), Date::new(7, 2, 2023)),
                ),
            },
            Destination {
                iata: "YYC".to_string(),
                dates: DateRange(
                    SingleDateRange::DateRange(Date::new(3, 2, 2023), Date::new(7, 2, 2023)),
                    SingleDateRange::DateRange(Date::new(4, 2, 2023), Date::new(7, 2, 2023)),
                ),
            },
            Destination {
                iata: "SEA".to_string(),
                dates: DateRange(
                    SingleDateRange::DateRange(Date::new(5, 2, 2023), Date::new(7, 2, 2023)),
                    SingleDateRange::DateRange(Date::new(6, 2, 2023), Date::new(7, 2, 2023)),
                ),
            },
            Destination {
                iata: "FEA".to_string(),
                dates: DateRange(
                    SingleDateRange::FixedDate(Date::new(8, 2, 2023)),
                    SingleDateRange::None,
                ),
            },
        ];

        let curr_node = FlightNode {
            flight: Flight {
                src: "YYZ".to_string(),
                dest: "YVR".to_string(),
                date: Date::new(2, 2, 2023),
            },
            back_price: Some(100.0),
            price: Some(100.0),
            prev: Some(Rc::new(FlightNode {
                flight: Flight {
                    src: "".to_string(),
                    dest: "YYZ".to_string(),
                    date: Date::new(1, 2, 2023),
                },
                back_price: Some(200.0),
                price: Some(100.0),
                prev: None,
                dest_ref: Destination {
                    iata: "YYZ".to_string(),
                    dates: DateRange(SingleDateRange::None, SingleDateRange::None),
                },
            })),
            dest_ref: Destination {
                iata: "YVR".to_string(),
                dates: DateRange(SingleDateRange::None, SingleDateRange::None),
            },
        };

        let final_dest = Destination {
            iata: "YYZ".to_string(),
            dates: DateRange(SingleDateRange::None, SingleDateRange::None),
        };

        let dest_list = router.fill_dest_list(Rc::new(curr_node), &final_dest, &init_dest_list);

        assert!(dest_list
            .iter()
            .find(|d| d.iata == "YYC".to_string())
            .is_some());
        assert!(dest_list
            .iter()
            .find(|d| d.iata == "SEA".to_string())
            .is_some());
        assert!(dest_list
            .iter()
            .find(|d| d.iata == "FEA".to_string())
            .is_some());

        assert_eq!(dest_list.len(), 3);
    }

    #[tokio::test]
    async fn test_graph_search() {
        let mut router = Router::<TestPriceApiQuery>::new();

        // Setup router problem
        let problem = RouterProblem {
            dest_list: vec![
                Destination {
                    // Source
                    iata: "YYZ".to_string(),
                    dates: DateRange(
                        SingleDateRange::None,
                        SingleDateRange::DateRange(Date::new(1, 2, 2023), Date::new(3, 2, 2023)),
                    ),
                },
                Destination {
                    iata: "YVR".to_string(),
                    dates: DateRange(
                        SingleDateRange::DateRange(Date::new(2, 2, 2023), Date::new(4, 2, 2023)),
                        SingleDateRange::DateRange(Date::new(4, 2, 2023), Date::new(8, 2, 2023)),
                    ),
                },
                Destination {
                    iata: "YYC".to_string(),
                    dates: DateRange(
                        SingleDateRange::DateRange(Date::new(3, 2, 2023), Date::new(7, 2, 2023)),
                        SingleDateRange::DateRange(Date::new(4, 2, 2023), Date::new(8, 2, 2023)),
                    ),
                },
                Destination {
                    iata: "SEA".to_string(),
                    dates: DateRange(
                        SingleDateRange::DateRange(Date::new(5, 2, 2023), Date::new(7, 2, 2023)),
                        SingleDateRange::DateRange(Date::new(6, 2, 2023), Date::new(8, 2, 2023)),
                    ),
                },
                Destination {
                    iata: "YYZ".to_string(),
                    dates: DateRange(
                        SingleDateRange::FixedDate(Date::new(8, 2, 2023)),
                        SingleDateRange::None,
                    ),
                },
            ],
        };

        let result = router.calc(problem).await;
        println!("Result: {}", result);
        println!("Total price: ${}", result.total_price());
        println!("Stats: {}", router.stats);
    }
}
