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

    /// Date range for either the inbound or outbound flight, flexibility on whether the user wants
    /// exact dates, or doesn't card
    pub enum SingleDateRange {
        Fixed(Date),
        Anytime,
        Tolerance(Date, u8),
    }

    /// Contains the inbound and outbound dates for a flight, or the number of days the user wants
    pub enum DateRange {
        Date(SingleDateRange, SingleDateRange),
        NumberOfDays(u16),
    }

    /// Represents a single destination, as the IATA (airport code), and a date range which gives
    /// flexibility on when the user wants to go
    pub struct Destination {
        iata: String,
        dates: DateRange,
    }

    /// Represents a flight on a given day
    #[derive(Eq, PartialEq, Hash)]
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
