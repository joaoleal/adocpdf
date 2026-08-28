//! Where the date comes from.

use std::time::{SystemTime, UNIX_EPOCH};

use adocpdf_adapters::calendar::date_from_unix_days;
use adocpdf_domain::ports::{Clock, Date};

/// Reads the date from the host.
///
/// Used when a render is meant to carry the day it was produced. Any render
/// that must be reproducible should be given a [`FixedClock`] instead — that
/// choice belongs to the caller, which is the whole reason this is a port.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl SystemClock {
    /// Creates a clock reading the host.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    fn today(&self) -> Date {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());

        date_from_unix_days(i64::try_from(seconds / 86_400).unwrap_or(0))
    }
}

/// Returns the same date however often it is asked.
///
/// This is not only a test double. A reproducible build supplies a fixed date
/// deliberately, so that the same source yields the same bytes whenever it is
/// rendered.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    date: Date,
}

impl FixedClock {
    /// Creates a clock that always reports `date`.
    #[must_use]
    pub fn new(date: Date) -> Self {
        Self { date }
    }
}

impl Clock for FixedClock {
    fn today(&self) -> Date {
        self.date
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fixed_clock_reports_the_same_date_every_time() {
        let date = Date::new(2026, 8, 16).unwrap();
        let clock = FixedClock::new(date);

        assert_eq!(clock.today(), date);
        assert_eq!(clock.today(), date);
        assert_eq!(clock.today(), date);
    }

    #[test]
    fn the_system_clock_reports_a_plausible_date() {
        let today = SystemClock::new().today();

        assert!(
            today.year >= 2024,
            "the host clock should be somewhere in this century, got {today:?}"
        );
        assert!((1..=12).contains(&today.month));
        assert!((1..=31).contains(&today.day));
    }
}
