pub mod Queries {
    #[derive(Eq, PartialEq, Hash, Clone)]
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

    impl std::ops::Add<i32> for Date {
        type Output = Date;

        fn add(mut self, rhs: i32) -> Date {
            let num_days = match self.month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                2 => {
                    if (self.year % 4 == 0) {
                        29
                    } else {
                        28
                    }
                }
                _ => 30,
            };

            if self.day <= num_days - (rhs - 1) {
                self.day += rhs;
            } else if self.month < 11 {
                self.day = 1;
                self.month += 1;
            } else {
                self.day = 1;
                self.month = 0;
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

    pub struct SingleDateRangeIter {}

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

    impl Iterator for SingleDateRange {
        type Item = Date;

        fn next(&mut self) -> Option<Self::Item> {
            None
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
