use chrono::{Datelike, Local, Month, NaiveDate};

/// Calendar state for navigation and current view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarState {
    year:  i32,
    month: u32,
}

impl Default for CalendarState {
    fn default() -> Self {
        let now = Local::now();
        Self {
            year:  now.year(),
            month: now.month(),
        }
    }
}

impl CalendarState {
    /// Creates calendar state for current month.
    pub fn current() -> Self {
        Self::default()
    }

    /// Creates calendar state for specific year and month.
    ///
    /// # Errors
    ///
    /// Returns `CalendarError::InvalidMonth` if month is not in range 1-12.
    pub fn new(year: i32, month: u32) -> Result<Self, CalendarError> {
        if !(1..=12).contains(&month) {
            return Err(CalendarError::InvalidMonth { month });
        }
        Ok(Self { year, month })
    }

    /// Returns current year.
    pub fn year(&self) -> i32 {
        self.year
    }

    /// Returns current month (1-12).
    pub fn month(&self) -> u32 {
        self.month
    }

    /// Navigates to previous month.
    pub fn previous_month(&mut self) {
        if self.month == 1 {
            self.month = 12;
            self.year -= 1;
        } else {
            self.month -= 1;
        }
    }

    /// Navigates to next month.
    pub fn next_month(&mut self) {
        if self.month == 12 {
            self.month = 1;
            self.year += 1;
        } else {
            self.month += 1;
        }
    }

    /// Returns month name as string.
    pub fn month_name(&self) -> &'static str {
        Month::try_from(self.month as u8)
            .map(|m| m.name())
            .unwrap_or("Unknown")
    }

    /// Generates calendar data for current state.
    pub fn generate_calendar(&self) -> CalendarData {
        CalendarData::generate(self.year, self.month)
    }
}

/// Calendar day information for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayInfo {
    pub day:        u32,
    pub is_current: bool,
    pub is_today:   bool,
    pub in_month:   bool,
}

/// Generated calendar data for rendering a month view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarData {
    pub days: Vec<DayInfo>,
}

impl CalendarData {
    /// Generates calendar data for given year and month.
    ///
    /// Creates a 7x6 grid (42 days) including days from previous/next months
    /// to fill the calendar grid. Starts week on Monday.
    pub fn generate(year: i32, month: u32) -> Self {
        let today = Local::now().date_naive();

        let first_day = NaiveDate::from_ymd_opt(year, month, 1)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, 1, 1).expect("fallback date"));

        let weekday = first_day.weekday().num_days_from_monday();

        let days_in_month = Self::days_in_month(year, month);
        let prev_month_days = if month == 1 {
            Self::days_in_month(year - 1, 12)
        } else {
            Self::days_in_month(year, month - 1)
        };

        let mut days = Vec::with_capacity(42);

        for i in 0..weekday {
            let day = prev_month_days - weekday + i + 1;
            days.push(DayInfo {
                day,
                is_current: false,
                is_today:   false,
                in_month:   false,
            });
        }

        for day in 1..=days_in_month {
            let date = NaiveDate::from_ymd_opt(year, month, day).unwrap_or(first_day);
            let is_today = date == today;

            days.push(DayInfo {
                day,
                is_current: is_today,
                is_today,
                in_month:   true,
            });
        }

        let remaining = 42 - days.len();
        for day in 1..=remaining {
            days.push(DayInfo {
                day:        day as u32,
                is_current: false,
                is_today:   false,
                in_month:   false,
            });
        }

        Self { days }
    }

    fn days_in_month(year: i32, month: u32) -> u32 {
        NaiveDate::from_ymd_opt(year, month, 1)
            .and_then(|date| {
                if month == 12 {
                    NaiveDate::from_ymd_opt(year + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(year, month + 1, 1)
                }
                .map(|next| next.signed_duration_since(date).num_days() as u32)
            })
            .unwrap_or(30)
    }
}

/// Errors that can occur when working with calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarError {
    /// Month value is invalid (must be 1-12).
    InvalidMonth { month: u32 },
}

impl std::fmt::Display for CalendarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalendarError::InvalidMonth { month } => {
                write!(f, "invalid month: {}, must be in range 1-12", month)
            }
        }
    }
}

impl std::error::Error for CalendarError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_state_default_is_current_month() {
        let state = CalendarState::default();
        let now = Local::now();
        assert_eq!(state.year(), now.year());
        assert_eq!(state.month(), now.month());
    }

    #[test]
    fn calendar_state_new_validates_month() {
        assert!(CalendarState::new(2024, 1).is_ok());
        assert!(CalendarState::new(2024, 12).is_ok());
        assert!(CalendarState::new(2024, 0).is_err());
        assert!(CalendarState::new(2024, 13).is_err());
    }

    #[test]
    fn calendar_state_previous_month_wraps_year() {
        let mut state = CalendarState::new(2024, 1).expect("valid month");
        state.previous_month();
        assert_eq!(state.year(), 2023);
        assert_eq!(state.month(), 12);
    }

    #[test]
    fn calendar_state_previous_month_decrements() {
        let mut state = CalendarState::new(2024, 3).expect("valid month");
        state.previous_month();
        assert_eq!(state.year(), 2024);
        assert_eq!(state.month(), 2);
    }

    #[test]
    fn calendar_state_next_month_wraps_year() {
        let mut state = CalendarState::new(2024, 12).expect("valid month");
        state.next_month();
        assert_eq!(state.year(), 2025);
        assert_eq!(state.month(), 1);
    }

    #[test]
    fn calendar_state_next_month_increments() {
        let mut state = CalendarState::new(2024, 3).expect("valid month");
        state.next_month();
        assert_eq!(state.year(), 2024);
        assert_eq!(state.month(), 4);
    }

    #[test]
    fn calendar_state_month_name() {
        let state = CalendarState::new(2024, 1).expect("valid month");
        assert_eq!(state.month_name(), "January");

        let state = CalendarState::new(2024, 12).expect("valid month");
        assert_eq!(state.month_name(), "December");
    }

    #[test]
    fn calendar_data_generates_42_days() {
        let data = CalendarData::generate(2024, 10);
        assert_eq!(data.days.len(), 42);
    }

    #[test]
    fn calendar_data_october_2024_starts_on_tuesday() {
        let data = CalendarData::generate(2024, 10);

        assert!(!data.days[0].in_month);

        assert!(data.days[1].in_month);
        assert_eq!(data.days[1].day, 1);
    }

    #[test]
    fn calendar_data_marks_current_days() {
        let data = CalendarData::generate(2024, 10);
        let in_month_days: Vec<_> = data.days.iter().filter(|d| d.in_month).collect();
        assert_eq!(in_month_days.len(), 31);
    }

    #[test]
    fn calendar_data_february_2024_has_29_days() {
        let data = CalendarData::generate(2024, 2);
        let in_month_days: Vec<_> = data.days.iter().filter(|d| d.in_month).collect();
        assert_eq!(in_month_days.len(), 29);
    }

    #[test]
    fn calendar_data_february_2023_has_28_days() {
        let data = CalendarData::generate(2023, 2);
        let in_month_days: Vec<_> = data.days.iter().filter(|d| d.in_month).collect();
        assert_eq!(in_month_days.len(), 28);
    }
}
