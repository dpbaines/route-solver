pub mod queries {
    use chrono::{Days, NaiveDate, Duration};
    use std::{
        cmp::{max, min},
        rc::Rc,
    };
    use serde::{Deserialize, Serialize};

    pub type Date = NaiveDate;

    #[derive(Serialize, Deserialize)]
    pub struct EchoQuery {
        pub input: String
    }

    #[derive(Serialize, Deserialize)]
    pub struct RouteQuery {
        pub start_city: String,
        pub end_city: String,
        pub hops: Vec<String>,
    }

    /// Date range for either the inbound or outbound flight, flexibility on whether the user wants
    /// exact dates, or doesn't card
    #[derive(Debug, Eq, PartialEq, Hash, Clone)]
    pub enum SingleDateRange {
        None,
        FixedDate(Date),
        DateRange(Date, Date),
    }

    #[derive(Debug)]
    pub struct SingleDateRangeIter {
        date_range: SingleDateRange,
        src_date: Option<Date>,
        curr_date: Date,
        day_count: u16,
        restrictions: Rc<DateRestrictions>,
    }

    impl SingleDateRange {
        pub fn first_date(&self) -> Option<Date> {
            match &self {
                Self::None => None,
                Self::FixedDate(d) => Some(d.clone()),
                Self::DateRange(d1, _) => Some(d1.clone())
            }
        }

        pub fn last_date(&self) -> Option<Date> {
            match &self {
                Self::None => None,
                Self::FixedDate(d) => Some(d.clone()),
                Self::DateRange(_, d2) => Some(d2.clone())
            }
        }

        pub fn low_high(&self) -> (Option<Date>, Option<Date>) {
            (self.first_date(), self.last_date())
        }

        pub fn fixify(&self) -> Option<Self> {
            // Temporary solution
            match self {
                SingleDateRange::FixedDate(d) => Some(SingleDateRange::DateRange(d.clone(), d.clone())),
                SingleDateRange::DateRange(d1, d2) => Some(SingleDateRange::DateRange(d1.clone(), d2.clone())),
                SingleDateRange::None => None,
            }
        }

        pub fn iter(&self, restrictions: Rc<DateRestrictions>) -> SingleDateRangeIter {
            self.iter_partial(restrictions, self.first_date())
        }

        pub fn iter_partial(
            &self,
            restrictions: Rc<DateRestrictions>,
            src_date: Option<Date>,
        ) -> SingleDateRangeIter {
            let start_date = max(src_date.map(|d| d + restrictions.min_days.unwrap_or(Duration::days(0))), self.first_date());
            match self {
                Self::FixedDate(d) => SingleDateRangeIter {
                    date_range: self.clone(),
                    curr_date: start_date.unwrap(), // An error here is a hard error
                    day_count: 0,
                    src_date,
                    restrictions,
                },
                Self::DateRange(d1, _) => SingleDateRangeIter {
                    date_range: self.clone(),
                    curr_date: start_date.unwrap(),
                    day_count: 0,
                    src_date,
                    restrictions,
                },
                _ => SingleDateRangeIter {
                    date_range: self.clone(),
                    curr_date: NaiveDate::MIN,
                    day_count: 0,
                    src_date,
                    restrictions,
                },
            }
        }

        pub fn intersect(&self, other: &SingleDateRange) -> SingleDateRange {
            let (s_maybe_low, s_maybe_high) = self.low_high();
            let (o_maybe_low, o_maybe_high) = other.low_high();

            let s_maybe_low = s_maybe_low.or(o_maybe_low);
            let o_maybe_low = o_maybe_low.or(s_maybe_low);
            let s_maybe_high = s_maybe_high.or(o_maybe_high);
            let o_maybe_high = o_maybe_high.or(s_maybe_high);

            let dates = [s_maybe_low, s_maybe_high, o_maybe_low, o_maybe_high];
            if dates.iter().any(|f| f.is_none()) { return SingleDateRange::None }

            let lowest_date = max(s_maybe_low, o_maybe_low);
            let high_date = min(s_maybe_high, o_maybe_high);

            if lowest_date > high_date { return SingleDateRange::None }

            if lowest_date == high_date {
                SingleDateRange::FixedDate(lowest_date.unwrap())
            } else {
                SingleDateRange::DateRange(lowest_date.unwrap(), high_date.unwrap())
            }
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
        pub min_days: Option<Duration>,
        pub max_days: Option<Duration>,
    }

    impl Default for DateRestrictions {
        fn default() -> Self {
            Self { min_days: None, max_days: None }
        }
    }

    impl DateRestrictions {
        fn new() -> Self {
            DateRestrictions {
                min_days: None,
                max_days: None,
            }
        }

        fn add_min_days_constraint(&mut self, md: Duration) {
            self.min_days = Some(md);
        }

        fn add_max_days_constraint(&mut self, md: Duration) {
            self.max_days = Some(md);
        }

        fn within_constraints(&self, prev_date: Date, curr_date: Date) -> bool {
            let dur = curr_date - prev_date;
            let min_met = if let Some(md) = &self.min_days {
                dur >= *md
            } else {
                true
            };

            let max_met = if let Some(md) = &self.max_days {
                dur <= *md
            } else {
                true
            };

            min_met && max_met
        }
    }

    #[derive(Clone, Debug)]
    pub struct DateConstraints {
        pub date_range: Option<DateRange>,
        pub date_restrictions: Rc<DateRestrictions>,
    }

    impl DateConstraints {
        pub fn get_intersect_iter_with_next(&self, next: &DateConstraints, src_date: Option<Date>) -> SingleDateRangeIter {
            let drs = (self.date_range.clone(), next.date_range.clone());
            let sdr_intersect = match drs {
                (Some(dr1), Some(dr2)) => dr1.1.intersect(&dr2.0),
                (Some(dr1), None) => dr1.1,
                (None, Some(dr2)) => dr2.0,
                (None, None) => panic!("No date ranges should have been filtered and corrected by frontend")
            };

            sdr_intersect.iter_partial(self.date_restrictions.clone(), src_date) 
        }
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
            // TODO: Having a seperate fixed date and None date type is in retrospect really stupid, fix this later
            let end_date = match self.date_range.fixify() {
                Some(SingleDateRange::DateRange(_, d)) => d,
                _ => return None
            };

            // Check max restriction
            if self.restrictions.max_days.zip(self.src_date).map(|(max_days, src_date)| max_days <= self.curr_date.signed_duration_since(src_date)).unwrap_or(false) {
                return None; 
            }

            // Check if is past max date
            if self.curr_date > end_date {
                return None;
            }

            let ret = Some(self.curr_date);
            self.curr_date = self.curr_date + Days::new(1);
            ret
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

/**
 * Unit Tests
 */

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::queries::{Date, DateRestrictions, SingleDateRange};
    use chrono::{Days, Duration};

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
            min_days: Some(Duration::days(2)),
            max_days: Some(Duration::days(4)),
        });
        let d_range = SingleDateRange::DateRange(
            Date::from_ymd_opt(2023, 3, 3).unwrap(),
            Date::from_ymd_opt(2023, 3, 10).unwrap(),
        );
        let mut d_r_iter = d_range.iter(restrictions);

        println!("Iter dump: {:?}", d_r_iter);

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
