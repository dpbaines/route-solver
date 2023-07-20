pub mod queries {
    use std::cmp::Ordering;

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
    #[derive(Eq, PartialEq, Hash, Clone)]
    pub enum SingleDateRange {
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
            }
        }
    }

    /// Contains the inbound and outbound dates for a flight, or the number of days the user wants
    #[derive(Clone)]
    pub enum DateRange {
        Date(SingleDateRange, SingleDateRange),
        NumberOfDays(u16),
    }

    /// Represents a single destination, as the IATA (airport code), and a date range which gives
    /// flexibility on when the user wants to go
    #[derive(Clone)]
    pub struct Destination {
        iata: String,
        dates: DateRange,
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
            }
        }
    }

    /// Represents a flight on a given day
    #[derive(Eq, PartialEq, Hash)]
    pub struct Flight {
        pub src: String,
        pub dest: String,
        pub date: SingleDateRange,
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
}
