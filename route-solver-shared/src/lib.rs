pub mod queries {
    use std::cmp::{max, min, Ordering};

    #[derive(Debug, Eq, PartialEq, Hash, Clone)]
    pub struct Date {
        pub day: u16,
        pub month: u16,
        pub year: u16,
    }

    impl Date {
        pub fn new(day: u16, month: u16, year: u16) -> Date {
            Date { day, month, year }
        }
    }

    impl std::cmp::PartialOrd<Self> for Date {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            if self.year > other.year {
                return Some(Ordering::Greater);
            } else if self.year < other.year {
                return Some(Ordering::Less);
            }

            if self.month > other.month {
                return Some(Ordering::Greater);
            } else if self.month < other.month {
                return Some(Ordering::Less);
            }

            if self.day > other.day {
                return Some(Ordering::Greater);
            } else if self.day < other.day {
                return Some(Ordering::Less);
            }

            Some(Ordering::Equal)
        }
    }

    impl std::cmp::Ord for Date {
        fn cmp(&self, other: &Self) -> Ordering {
            return self.partial_cmp(other).unwrap();
        }
    }

    impl std::ops::Add<u16> for Date {
        type Output = Date;

        fn add(mut self, rhs: u16) -> Date {
            assert!(rhs < 28); // I don't want to deal with this case

            let num_days = match self.month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                2 => {
                    if self.year % 4 == 0 {
                        29
                    } else {
                        28
                    }
                }

                _ => 30,
            };

            let remaining_days = num_days - self.day;

            if self.day < num_days - (rhs - 1) {
                self.day += rhs;
            } else if self.month < 11 {
                self.day = rhs - remaining_days;
                self.month += 1;
            } else {
                self.day = rhs - remaining_days;
                self.month = 1;
                self.year += 1;
            }

            self
        }
    }

    /// Date range for either the inbound or outbound flight, flexibility on whether the user wants
    /// exact dates, or doesn't card
    #[derive(Debug, Eq, PartialEq, Hash, Clone)]
    pub enum SingleDateRange {
        None,
        FixedDate(Date),
        DateRange(Date, Date),
    }

    pub struct SingleDateRangeIter<'a> {
        date_range: &'a SingleDateRange,
        curr_date: Option<Date>,
    }

    impl SingleDateRange {
        pub fn iter(&self) -> SingleDateRangeIter {
            match self {
                Self::FixedDate(d) => SingleDateRangeIter {
                    date_range: &self,
                    curr_date: Some(d.clone()),
                },
                Self::DateRange(d1, _) => SingleDateRangeIter {
                    date_range: &self,
                    curr_date: Some(d1.clone()),
                },
                _ => SingleDateRangeIter {
                    date_range: &self,
                    curr_date: None,
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
                        return SingleDateRange::DateRange(max(d1.clone(), date + 1), d2.clone());
                    } else {
                        return SingleDateRange::None;
                    }
                }
                Self::None => Self::None,
            }
        }
    }

    /// Contains the inbound and outbound dates for a flight, or the number of days the user wants
    #[derive(Clone)]
    pub struct DateRange(pub SingleDateRange, pub SingleDateRange);

    /// Represents a single destination, as the IATA (airport code), and a date range which gives
    /// flexibility on when the user wants to go
    #[derive(Clone)]
    pub struct Destination {
        pub iata: String,
        pub dates: DateRange,
    }

    impl<'a> Iterator for SingleDateRangeIter<'a> {
        type Item = Date;

        fn next(&mut self) -> Option<Self::Item> {
            match self.date_range {
                SingleDateRange::FixedDate(_) => {
                    if let Some(d) = self.curr_date.clone() {
                        self.curr_date = None;
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
                            self.curr_date = Some(d.clone() + 1);
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
    #[derive(Eq, PartialEq, Hash, Clone)]
    pub struct Flight {
        pub src: String,
        pub dest: String,
        pub date: Date,
    }

    pub struct FlightPrice {
        flight: Flight,
        price: f32,
    }
}
#[cfg(test)]
mod tests {
    use crate::queries::{Date, SingleDateRange};

    #[test]
    fn test_date_adding() {
        let d1 = Date::new(1, 2, 2023);
        assert_eq!(d1 + 4, Date::new(5, 2, 2023));

        let d2 = Date::new(30, 1, 2023);
        assert_eq!(d2 + 2, Date::new(1, 2, 2023));

        let d3 = Date::new(30, 12, 2023);
        assert_eq!(d3 + 5, Date::new(4, 1, 2024));

        let d4 = Date::new(27, 2, 2024);
        assert_eq!(d4 + 3, Date::new(1, 3, 2024));
    }

    #[test]
    fn test_date_cmp() {
        let d1 = Date::new(1, 2, 2023);
        let d2 = Date::new(2, 2, 2023);
        let d3 = Date::new(1, 3, 2023);
        let d4 = Date::new(1, 2, 2024);

        assert!(d1 < d2);
        assert!(d2 > d1);

        assert!(d3 > d2);
        assert!(d4 > d3);
    }

    #[test]
    fn test_date_range_iter() {
        let d_fixed_range = SingleDateRange::FixedDate(Date::new(3, 3, 2023));
        let mut d_iter = d_fixed_range.iter();

        assert_eq!(d_iter.next(), Some(Date::new(3, 3, 2023)));
        assert_eq!(d_iter.next(), None);

        let d_range = SingleDateRange::DateRange(Date::new(3, 3, 2023), Date::new(5, 3, 2023));
        let mut d_r_iter = d_range.iter();

        assert_eq!(d_r_iter.next(), Some(Date::new(3, 3, 2023)));
        assert_eq!(d_r_iter.next(), Some(Date::new(4, 3, 2023)));
        assert_eq!(d_r_iter.next(), Some(Date::new(5, 3, 2023)));
        assert_eq!(d_r_iter.next(), None);
    }

    #[test]
    fn test_date_range_intersect() {
        let d_fixed_fixed_no1 = SingleDateRange::FixedDate(Date::new(3, 3, 2023));
        let d_fixed_fixed_no2 = SingleDateRange::FixedDate(Date::new(4, 3, 2023));

        assert_eq!(
            d_fixed_fixed_no1.intersect(&d_fixed_fixed_no2),
            SingleDateRange::None
        );

        let d_fixed_fixed1 = SingleDateRange::FixedDate(Date::new(3, 3, 2023));
        let d_fixed_fixed2 = SingleDateRange::FixedDate(Date::new(3, 3, 2023));

        assert_eq!(
            d_fixed_fixed1.intersect(&d_fixed_fixed2),
            SingleDateRange::FixedDate(Date::new(3, 3, 2023))
        );

        let d_none1 = SingleDateRange::None;
        let d_none2 = SingleDateRange::FixedDate(Date::new(3, 3, 2023));

        assert_eq!(d_none1.intersect(&d_none2), SingleDateRange::None);

        let d_range_range1 =
            SingleDateRange::DateRange(Date::new(3, 3, 2023), Date::new(10, 3, 2023));
        let d_range_range2 =
            SingleDateRange::DateRange(Date::new(6, 3, 2023), Date::new(18, 3, 2023));

        assert_eq!(
            d_range_range1.intersect(&d_range_range2),
            SingleDateRange::DateRange(Date::new(6, 3, 2023), Date::new(10, 3, 2023))
        );

        let d_range_range_subset1 =
            SingleDateRange::DateRange(Date::new(3, 3, 2023), Date::new(18, 3, 2023));
        let d_range_range_subset2 =
            SingleDateRange::DateRange(Date::new(6, 3, 2023), Date::new(10, 3, 2023));

        assert_eq!(
            d_range_range_subset1.intersect(&d_range_range_subset2),
            SingleDateRange::DateRange(Date::new(6, 3, 2023), Date::new(10, 3, 2023))
        );
    }
}
