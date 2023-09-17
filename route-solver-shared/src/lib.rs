pub mod queries {
    use chrono::{Days, NaiveDate};
    use std::{
        cmp::{max, min},
        rc::Rc,
    };

    pub type Date = NaiveDate;

    /// Date range for either the inbound or outbound flight, flexibility on whether the user wants
    /// exact dates, or doesn't card
    #[derive(Debug, Eq, PartialEq, Hash, Clone)]
    pub enum SingleDateRange {
        None,
        FixedDate(Date),
        DateRange(Date, Date),
    }

    /// Wrapper struct representing a flat number of days
    #[derive(Clone, Debug)]
    pub struct NumDays(pub u16);

    pub struct SingleDateRangeIter {
        date_range: SingleDateRange,
        src_date: Option<Date>,
        curr_date: Option<Date>,
        day_count: u16,
        restrictions: Rc<DateRestrictions>,
    }

    impl SingleDateRange {
        pub fn iter(
            &self,
            restrictions: Rc<DateRestrictions>,
            src_date: Option<Date>,
        ) -> SingleDateRangeIter {
            match self {
                Self::FixedDate(d) => SingleDateRangeIter {
                    date_range: self.clone(),
                    curr_date: Some(d.clone()),
                    day_count: 0,
                    src_date,
                    restrictions,
                },
                Self::DateRange(d1, _) => SingleDateRangeIter {
                    date_range: self.clone(),
                    curr_date: Some(d1.clone()),
                    day_count: 0,
                    src_date,
                    restrictions,
                },
                _ => SingleDateRangeIter {
                    date_range: self.clone(),
                    curr_date: None,
                    day_count: 0,
                    src_date,
                    restrictions,
                },
            }
        }

        pub fn get_low_high(&self) -> Option<(Date, Date)> {
            let low_s = match self {
                SingleDateRange::FixedDate(d) => d,
                SingleDateRange::DateRange(d1, _) => d1,
                SingleDateRange::None => return None,
            };

            let high_s = match self {
                SingleDateRange::FixedDate(d) => d,
                SingleDateRange::DateRange(_, d2) => d2,
                SingleDateRange::None => return None,
            };

            Some((low_s.clone(), high_s.clone()))
        }

        pub fn intersect(&self, other: &SingleDateRange) -> SingleDateRange {
            let s_hl = self.get_low_high();
            let o_hl = other.get_low_high();

            let (self_low, self_high) = match s_hl {
                Some((h, l)) => (h, l),
                None => return SingleDateRange::None,
            };

            let (other_low, other_high) = match o_hl {
                Some((h, l)) => (h, l),
                None => return SingleDateRange::None,
            };

            let low_inter = max(self_low, other_low);
            let high_inter = min(self_high, other_high);

            if low_inter > high_inter {
                return SingleDateRange::None;
            };

            if low_inter == high_inter {
                return SingleDateRange::FixedDate(low_inter);
            };

            SingleDateRange::DateRange(low_inter, high_inter)
        }

        /// Given a date truncate all dates before (inclusive) the given date.
        pub fn truncate(&self, date: Date) -> Self {
            match self {
                Self::FixedDate(d) => {
                    if *d > date {
                        return SingleDateRange::FixedDate(d.clone());
                    } else {
                        return SingleDateRange::None;
                    }
                }
                Self::DateRange(d1, d2) => {
                    if *d2 > date {
                        // Don't particularly care if we return a date range where before and after are the same day
                        // Shouldn't cause issues, but if it does fix here
                        return SingleDateRange::DateRange(
                            max(d1.clone(), date + Days::new(1)),
                            d2.clone(),
                        );
                    } else {
                        return SingleDateRange::None;
                    }
                }
                Self::None => Self::None,
            }
        }
    }

    /// Contains the inbound and outbound dates for a flight, or the number of days the user wants
    #[derive(Clone, Debug)]
    pub struct DateRange(pub SingleDateRange, pub SingleDateRange);

    #[derive(Clone, Debug)]
    pub struct DateRestrictions {
        pub min_days: Option<NumDays>,
        pub max_days: Option<NumDays>,
    }

    impl DateRestrictions {
        fn new() -> Self {
            DateRestrictions {
                min_days: None,
                max_days: None,
            }
        }

        fn add_min_days_constraint(&mut self, md: NumDays) {
            self.min_days = Some(md);
        }

        fn add_max_days_constraint(&mut self, md: NumDays) {
            self.max_days = Some(md);
        }

        fn within_constraints(&self, prev_date: Date, curr_date: Date) -> bool {
            let dur = curr_date - prev_date;
            let min_met = if let Some(md) = &self.min_days {
                dur.num_days() >= md.0 as i64
            } else {
                true
            };

            let max_met = if let Some(md) = &self.max_days {
                dur.num_days() <= md.0 as i64
            } else {
                true
            };

            min_met && max_met
        }
    }

    #[derive(Clone, Debug)]
    pub struct DateConstraints {
        date_range: DateRange,
        date_restrictions: Rc<DateRestrictions>,
    }

    /// Represents a single destination, as the IATA (airport code), and a date range which gives
    /// flexibility on when the user wants to go
    #[derive(Clone, Debug)]
    pub struct Destination {
        pub iata: String,
        pub dates: DateConstraints,
    }

    impl Iterator for SingleDateRangeIter {
        type Item = Date;

        fn next(&mut self) -> Option<Self::Item> {
            match &self.date_range {
                SingleDateRange::FixedDate(_) => {
                    if let Some(d) = self.curr_date.clone() {
                        self.curr_date = None;
                        if let Some(src_date) = self.src_date {
                            if self.restrictions.within_constraints(src_date, d) {
                                return None;
                            }
                        }
                        return Some(d.clone());
                    } else {
                        return None;
                    }
                }
                SingleDateRange::DateRange(_, date2) => {
                    if let Some(d) = self.curr_date.clone() {
                        if d > *date2 {
                            self.curr_date = None;
                            return None;
                        } else {
                            self.curr_date = Some(d.clone() + Days::new(1));
                            return Some(d);
                        }
                    } else {
                        return None;
                    }
                }
                SingleDateRange::None => return None,
            }
        }
    }

    /// Represents a flight on a given day
    #[derive(Debug, Eq, PartialEq, Hash, Clone)]
    pub struct Flight {
        pub src: String,
        pub dest: String,
        pub date: Date,
    }

    #[derive(Debug)]
    pub struct FlightPrice {
        pub flight: Flight,
        pub price: f32,
    }
}
#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::queries::{Date, DateRestrictions, NumDays, SingleDateRange};
    use chrono::Days;

    #[test]
    fn test_date_cmp() {
        let d1 = Date::from_ymd_opt(2023, 2, 1).unwrap();
        let d2 = Date::from_ymd_opt(2023, 2, 2).unwrap();
        let d3 = Date::from_ymd_opt(2023, 3, 1).unwrap();
        let d4 = Date::from_ymd_opt(2024, 2, 1).unwrap();

        assert!(d1 < d2);
        assert!(d2 > d1);

        assert!(d3 > d2);
        assert!(d4 > d3);
    }

    #[test]
    fn test_date_iter_with_restrictions() {
        let restrictions = Rc::new(DateRestrictions {
            min_days: Some(NumDays(2)),
            max_days: Some(NumDays(4)),
        });
        let d_range = SingleDateRange::DateRange(
            Date::from_ymd_opt(2023, 3, 3).unwrap(),
            Date::from_ymd_opt(2023, 3, 10).unwrap(),
        );
        let mut d_r_iter = d_range.iter(restrictions);

        assert_eq!(
            d_r_iter.next(),
            Some(Date::from_ymd_opt(2023, 3, 3).unwrap())
        );
        assert_eq!(
            d_r_iter.next(),
            Some(Date::from_ymd_opt(2023, 3, 4).unwrap())
        );
        assert_eq!(
            d_r_iter.next(),
            Some(Date::from_ymd_opt(2023, 3, 5).unwrap())
        );
        assert_eq!(
            d_r_iter.next(),
            Some(Date::from_ymd_opt(2023, 3, 6).unwrap())
        );
        assert_eq!(d_r_iter.next(), None);
    }

    #[test]
    fn test_date_range_iter() {
        let d_fixed_range = SingleDateRange::FixedDate(Date::from_ymd_opt(2023, 3, 3).unwrap());
        let mut d_iter = d_fixed_range.iter(Rc::new(DateRestrictions {
            min_days: None,
            max_days: None,
        }));

        assert_eq!(d_iter.next(), Some(Date::from_ymd_opt(2023, 3, 3).unwrap()));
        assert_eq!(d_iter.next(), None);

        let d_range = SingleDateRange::DateRange(
            Date::from_ymd_opt(2023, 3, 3).unwrap(),
            Date::from_ymd_opt(2023, 3, 5).unwrap(),
        );
        let mut d_r_iter = d_range.iter(Rc::new(DateRestrictions {
            min_days: None,
            max_days: None,
        }));

        assert_eq!(
            d_r_iter.next(),
            Some(Date::from_ymd_opt(2023, 3, 3).unwrap())
        );
        assert_eq!(
            d_r_iter.next(),
            Some(Date::from_ymd_opt(2023, 3, 4).unwrap())
        );
        assert_eq!(
            d_r_iter.next(),
            Some(Date::from_ymd_opt(2023, 3, 5).unwrap())
        );
        assert_eq!(d_r_iter.next(), None);
    }

    #[test]
    fn test_date_range_intersect() {
        let d_fixed_fixed_no1 = SingleDateRange::FixedDate(Date::from_ymd_opt(2023, 3, 3).unwrap());
        let d_fixed_fixed_no2 = SingleDateRange::FixedDate(Date::from_ymd_opt(2023, 3, 4).unwrap());

        assert_eq!(
            d_fixed_fixed_no1.intersect(&d_fixed_fixed_no2),
            SingleDateRange::None
        );

        let d_fixed_fixed1 = SingleDateRange::FixedDate(Date::from_ymd_opt(2023, 3, 3).unwrap());
        let d_fixed_fixed2 = SingleDateRange::FixedDate(Date::from_ymd_opt(2023, 3, 3).unwrap());

        assert_eq!(
            d_fixed_fixed1.intersect(&d_fixed_fixed2),
            SingleDateRange::FixedDate(Date::from_ymd_opt(2023, 3, 3).unwrap())
        );

        let d_none1 = SingleDateRange::None;
        let d_none2 = SingleDateRange::FixedDate(Date::from_ymd_opt(2023, 3, 3).unwrap());

        assert_eq!(d_none1.intersect(&d_none2), SingleDateRange::None);

        let d_range_range1 = SingleDateRange::DateRange(
            Date::from_ymd_opt(2023, 3, 3).unwrap(),
            Date::from_ymd_opt(2023, 3, 10).unwrap(),
        );
        let d_range_range2 = SingleDateRange::DateRange(
            Date::from_ymd_opt(2023, 3, 6).unwrap(),
            Date::from_ymd_opt(2023, 3, 18).unwrap(),
        );

        assert_eq!(
            d_range_range1.intersect(&d_range_range2),
            SingleDateRange::DateRange(
                Date::from_ymd_opt(2023, 3, 6).unwrap(),
                Date::from_ymd_opt(2023, 3, 10).unwrap()
            )
        );

        let d_range_range_subset1 = SingleDateRange::DateRange(
            Date::from_ymd_opt(2023, 3, 3).unwrap(),
            Date::from_ymd_opt(2023, 3, 18).unwrap(),
        );
        let d_range_range_subset2 = SingleDateRange::DateRange(
            Date::from_ymd_opt(2023, 3, 6).unwrap(),
            Date::from_ymd_opt(2023, 3, 10).unwrap(),
        );

        assert_eq!(
            d_range_range_subset1.intersect(&d_range_range_subset2),
            SingleDateRange::DateRange(
                Date::from_ymd_opt(2023, 3, 6).unwrap(),
                Date::from_ymd_opt(2023, 3, 10).unwrap()
            )
        );

        let d_range_range_shared1 = SingleDateRange::DateRange(
            Date::from_ymd_opt(2023, 3, 3).unwrap(),
            Date::from_ymd_opt(2023, 3, 6).unwrap(),
        );
        let d_range_range_shared2 = SingleDateRange::DateRange(
            Date::from_ymd_opt(2023, 3, 6).unwrap(),
            Date::from_ymd_opt(2023, 3, 10).unwrap(),
        );

        assert_eq!(
            d_range_range_shared1.intersect(&d_range_range_shared2),
            SingleDateRange::FixedDate(Date::from_ymd_opt(2023, 3, 6).unwrap())
        );
    }
}
