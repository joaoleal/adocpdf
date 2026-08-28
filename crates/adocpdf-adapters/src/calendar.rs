//! Converting between a calendar date and the epoch counts other agencies use.
//!
//! This is an adapter concern rather than a host one: it translates between a
//! representation the use case owns — a domain [`Date`] — and the one an
//! external agency wants, which is exactly what the interface-adapter ring is
//! for. Both directions live here together because they are inverses that have
//! to agree, and a rule enforced in two places is eventually enforced in one.

use adocpdf_domain::ports::Date;

/// Converts a calendar date into seconds since the Unix epoch, at midnight.
///
/// The AsciiDoc parser takes a reference time as a Unix timestamp, so the
/// injected date has to be expressible as one. Midnight is used because a
/// [`Date`] carries no time of day, and picking any other instant would invent
/// precision the caller never supplied.
#[must_use]
pub fn unix_timestamp(date: Date) -> i64 {
    days_from_civil(date) * 86_400
}

/// Converts a calendar date into days since the Unix epoch.
///
/// The inverse of [`date_from_unix_days`], using the same era-shifted
/// arithmetic, so the two agree by construction rather than by coincidence.
fn days_from_civil(date: Date) -> i64 {
    let year = i64::from(date.year) - i64::from(date.month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;

    let month = i64::from(date.month);
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(date.day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

/// Converts days since the Unix epoch into a calendar date.
///
/// Uses the civil-from-days algorithm, which is exact for the proleptic
/// Gregorian calendar and needs no lookup tables — so the conversion is the
/// same on every platform, which a locale-aware library call would not
/// guarantee.
#[must_use]
pub fn date_from_unix_days(days: i64) -> Date {
    // Shift the era so that the year starts in March, which puts the leap day
    // at the end of the year and removes it from the arithmetic.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = if month <= 2 { year + 1 } else { year };

    Date {
        year: i32::try_from(year).unwrap_or(0),
        month: u8::try_from(month).unwrap_or(1),
        day: u8::try_from(day).unwrap_or(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_the_first_of_january_nineteen_seventy() {
        assert_eq!(
            date_from_unix_days(0),
            Date::new(1970, 1, 1).unwrap(),
            "day zero anchors the whole conversion"
        );
    }

    #[test]
    fn a_known_day_converts_to_its_known_date() {
        // 2000-03-01 is 11017 days after the epoch, and sits immediately after
        // a leap day in a century year that *is* a leap year — the case the
        // arithmetic is most likely to get wrong.
        assert_eq!(date_from_unix_days(11_017), Date::new(2000, 3, 1).unwrap());
    }

    #[test]
    fn the_day_before_a_leap_day_is_the_leap_day() {
        assert_eq!(date_from_unix_days(11_016), Date::new(2000, 2, 29).unwrap());
    }

    #[test]
    fn a_non_leap_century_has_no_leap_day() {
        // 1900 was not a leap year, so 1900-03-01 follows 1900-02-28.
        let first_of_march = -25_508;

        assert_eq!(
            date_from_unix_days(first_of_march),
            Date::new(1900, 3, 1).unwrap()
        );
        assert_eq!(
            date_from_unix_days(first_of_march - 1),
            Date::new(1900, 2, 28).unwrap()
        );
    }

    #[test]
    fn converting_a_date_to_days_and_back_returns_it() {
        for date in [
            Date::new(1970, 1, 1).unwrap(),
            Date::new(2000, 2, 29).unwrap(),
            Date::new(1900, 2, 28).unwrap(),
            Date::new(2026, 8, 16).unwrap(),
            Date::new(2100, 12, 31).unwrap(),
        ] {
            assert_eq!(
                date_from_unix_days(days_from_civil(date)),
                date,
                "the two conversions must agree by construction"
            );
        }
    }

    #[test]
    fn the_epoch_is_timestamp_zero() {
        assert_eq!(unix_timestamp(Date::new(1970, 1, 1).unwrap()), 0);
    }

    #[test]
    fn a_timestamp_lands_on_midnight() {
        let timestamp = unix_timestamp(Date::new(2026, 8, 16).unwrap());

        assert_eq!(
            timestamp % 86_400,
            0,
            "a date carries no time of day, so it must not invent one"
        );
    }
}
