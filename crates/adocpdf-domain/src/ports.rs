//! The traits through which the application reaches the outside world.
//!
//! Every one of these is implemented in `adocpdf-infra` and injected at a
//! composition root. The domain names the contract; it never names the
//! technology satisfying it.

use adocpdf_core::theme::ThemeSet;

use crate::document_plan::LayoutPlan;
use crate::error::{DomainError, SourceLocation};
use crate::sandbox::SandboxedPath;

/// A calendar date, with no time and no zone.
///
/// Rendering needs a date and nothing finer, and a date carries no ambiguity
/// about which instant it refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    /// The year.
    pub year: i32,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month, 1 to 31.
    pub day: u8,
}

impl Date {
    /// Creates a date.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidDate`] when the month is outside 1 to 12, or the day
    /// outside 1 to 31. The check is deliberately coarse: this type exists to
    /// carry a date into the renderer, not to do calendar arithmetic.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, InvalidDate> {
        if !(1..=12).contains(&month) {
            return Err(InvalidDate::Month { month });
        }
        if !(1..=31).contains(&day) {
            return Err(InvalidDate::Day { day });
        }
        Ok(Self { year, month, day })
    }
}

/// A date that cannot exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidDate {
    /// The month was outside 1 to 12.
    #[error("month must be between 1 and 12, got {month}")]
    Month {
        /// The month that was proposed.
        month: u8,
    },
    /// The day was outside 1 to 31.
    #[error("day must be between 1 and 31, got {day}")]
    Day {
        /// The day that was proposed.
        day: u8,
    },
}

/// Supplies the date a render should use.
///
/// A port rather than a direct call to the host clock, because a render that
/// reads the wall clock cannot be reproducible. Both the layout engine and the
/// AsciiDoc parser want a date; both get it from here, so they agree.
pub trait Clock {
    /// The date to treat as today.
    fn today(&self) -> Date;
}

/// Reads and writes files inside the sandbox.
///
/// Every method takes a [`SandboxedPath`], so confinement is checked by the
/// type system rather than by remembering to call a validator.
pub trait SourceStore {
    /// Reads a source document as text.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InputNotFound`] when the file does not exist, or
    /// [`DomainError::InputUnreadable`] when it exists but cannot be read.
    fn read(&self, path: &SandboxedPath) -> Result<String, DomainError>;

    /// Writes the finished document.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::OutputUnwritable`] when the file cannot be
    /// created or written.
    fn write(&self, path: &SandboxedPath, bytes: &[u8]) -> Result<(), DomainError>;
}

/// A construct the parser understood but this renderer does not yet support.
///
/// Collected rather than treated as fatal: aborting on the first unsupported
/// construct would make the renderer useless against any real document. Every
/// one is reported so the omission is never silent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedConstruct {
    /// What was skipped, named the way an author would recognise it.
    pub construct: String,
    /// Where it appeared in the source.
    pub location: SourceLocation,
}

/// What a parse produced.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutcome {
    /// The document, containing only supported constructs.
    pub document: adocpdf_core::document::Document,
    /// Everything left out, in source order.
    pub skipped: Vec<SkippedConstruct>,
}

/// Turns AsciiDoc source into the document model.
pub trait DocumentParser {
    /// Parses `source`.
    ///
    /// `origin` names the document for diagnostics. `today` is the date the
    /// parser should use when resolving any date-dependent attribute, so that
    /// parsing is reproducible.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ParseFailed`] when the source is not valid
    /// AsciiDoc, or [`DomainError::ReferenceOutsideRoot`] when the document
    /// refers to a file outside the project root.
    fn parse(&self, source: &str, origin: &str, today: Date) -> Result<ParseOutcome, DomainError>;
}

/// Supplies the themes a document may use.
pub trait ThemeRepository {
    /// Loads the available themes.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidTheme`] when a theme's settings are not
    /// usable.
    fn load(&self) -> Result<ThemeSet, DomainError>;
}

/// Lays a planned document out and produces PDF bytes.
///
/// The renderer receives a [`LayoutPlan`] rather than a document and a theme
/// set: deciding which theme applies where is a business rule, already settled
/// before this port is called. What is left here is typesetting.
pub trait DocumentRenderer {
    /// Renders the plan to PDF bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::LayoutFailed`] when the document cannot be laid
    /// out.
    fn render(&self, plan: &LayoutPlan, origin: &str, today: Date) -> Result<Vec<u8>, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ordinary_date_is_accepted() {
        let date = Date::new(2026, 8, 16).unwrap();

        assert_eq!((date.year, date.month, date.day), (2026, 8, 16));
    }

    #[test]
    fn a_month_outside_the_year_is_rejected() {
        assert_eq!(Date::new(2026, 0, 1), Err(InvalidDate::Month { month: 0 }));
        assert_eq!(
            Date::new(2026, 13, 1),
            Err(InvalidDate::Month { month: 13 })
        );
    }

    #[test]
    fn a_day_outside_the_month_is_rejected() {
        assert_eq!(Date::new(2026, 1, 0), Err(InvalidDate::Day { day: 0 }));
        assert_eq!(Date::new(2026, 1, 32), Err(InvalidDate::Day { day: 32 }));
    }

    #[test]
    fn a_rejected_date_says_what_was_wrong() {
        assert!(
            Date::new(2026, 13, 1)
                .unwrap_err()
                .to_string()
                .contains("13")
        );
    }
}
