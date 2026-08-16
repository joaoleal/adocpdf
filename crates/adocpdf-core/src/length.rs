//! A physical length on the page.

use std::error::Error;
use std::fmt;

/// Points per inch, the definition typography and PDF both use.
const POINTS_PER_INCH: f64 = 72.0;

/// Millimeters per inch, exact by definition of the inch.
const MILLIMETERS_PER_INCH: f64 = 25.4;

/// A positive, finite physical length.
///
/// Lengths are stored in points, the unit PDF itself uses, so converting to the
/// output format never loses precision to a round trip. Construction validates,
/// which means an invalid length is not representable: code holding a `Length`
/// need not re-check it.
///
/// Zero is deliberately not a length. Every length in the theme model is a page
/// dimension, a margin, or a font size, and a zero value for any of those is a
/// mistake rather than a degenerate-but-valid case.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Length {
    points: f64,
}

impl Length {
    /// Creates a length from a measurement in points.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLength`] when the value is not finite, or is zero or
    /// negative.
    pub fn from_points(points: f64) -> Result<Self, InvalidLength> {
        if !points.is_finite() {
            return Err(InvalidLength::NotFinite);
        }
        if points <= 0.0 {
            return Err(InvalidLength::NotPositive);
        }
        Ok(Self { points })
    }

    /// Creates a length from a measurement in millimeters.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLength`] when the value is not finite, or is zero or
    /// negative.
    pub fn from_millimeters(millimeters: f64) -> Result<Self, InvalidLength> {
        Self::from_points(millimeters / MILLIMETERS_PER_INCH * POINTS_PER_INCH)
    }

    /// Creates a length from a measurement in inches.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLength`] when the value is not finite, or is zero or
    /// negative.
    pub fn from_inches(inches: f64) -> Result<Self, InvalidLength> {
        Self::from_points(inches * POINTS_PER_INCH)
    }

    /// This length in points.
    #[must_use]
    pub fn points(self) -> f64 {
        self.points
    }

    /// This length in millimeters.
    #[must_use]
    pub fn millimeters(self) -> f64 {
        self.points / POINTS_PER_INCH * MILLIMETERS_PER_INCH
    }

    /// This length in inches.
    #[must_use]
    pub fn inches(self) -> f64 {
        self.points / POINTS_PER_INCH
    }

    /// Adds two lengths.
    ///
    /// Always succeeds: the sum of two positive finite values is positive, and
    /// can only become non-finite by overflowing to infinity, which
    /// [`Length::from_points`] would have rejected on the way in.
    #[must_use]
    pub fn saturating_add(self, other: Self) -> Self {
        let sum = self.points + other.points;
        if sum.is_finite() {
            Self { points: sum }
        } else {
            Self { points: f64::MAX }
        }
    }
}

/// Why a proposed length is not usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidLength {
    /// The value was infinite or not a number.
    NotFinite,
    /// The value was zero or negative.
    NotPositive,
}

impl fmt::Display for InvalidLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite => f.write_str("length must be a finite number"),
            Self::NotPositive => f.write_str("length must be greater than zero"),
        }
    }
}

impl Error for InvalidLength {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_positive_measurement_becomes_a_length() {
        let length = Length::from_points(12.0).unwrap();

        assert!((length.points() - 12.0).abs() < f64::EPSILON);
    }

    #[test]
    fn zero_is_not_a_length() {
        assert_eq!(Length::from_points(0.0), Err(InvalidLength::NotPositive));
    }

    #[test]
    fn a_negative_measurement_is_not_a_length() {
        assert_eq!(Length::from_points(-1.0), Err(InvalidLength::NotPositive));
    }

    #[test]
    fn an_infinite_measurement_is_not_a_length() {
        assert_eq!(
            Length::from_points(f64::INFINITY),
            Err(InvalidLength::NotFinite)
        );
        assert_eq!(
            Length::from_points(f64::NEG_INFINITY),
            Err(InvalidLength::NotFinite)
        );
    }

    #[test]
    fn a_measurement_that_is_not_a_number_is_not_a_length() {
        assert_eq!(Length::from_points(f64::NAN), Err(InvalidLength::NotFinite));
    }

    #[test]
    fn an_inch_is_seventy_two_points() {
        let length = Length::from_inches(1.0).unwrap();

        assert!((length.points() - 72.0).abs() < 1e-9);
    }

    #[test]
    fn an_inch_is_twenty_five_point_four_millimeters() {
        let length = Length::from_inches(1.0).unwrap();

        assert!((length.millimeters() - 25.4).abs() < 1e-9);
    }

    #[test]
    fn converting_to_a_unit_and_back_preserves_the_length() {
        let original = Length::from_millimeters(210.0).unwrap();

        let round_tripped = Length::from_millimeters(original.millimeters()).unwrap();

        assert!((original.points() - round_tripped.points()).abs() < 1e-9);
    }

    #[test]
    fn rejection_explains_which_rule_was_broken() {
        assert_eq!(
            InvalidLength::NotPositive.to_string(),
            "length must be greater than zero"
        );
        assert_eq!(
            InvalidLength::NotFinite.to_string(),
            "length must be a finite number"
        );
    }

    #[test]
    fn lengths_compare_by_magnitude() {
        let smaller = Length::from_points(10.0).unwrap();
        let larger = Length::from_points(20.0).unwrap();

        assert!(smaller < larger);
    }

    #[test]
    fn adding_lengths_sums_them() {
        let sum = Length::from_points(10.0)
            .unwrap()
            .saturating_add(Length::from_points(5.0).unwrap());

        assert!((sum.points() - 15.0).abs() < f64::EPSILON);
    }
}
