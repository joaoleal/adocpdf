//! The `adocpdf` command-line entry point.
//!
//! A composition root: it parses arguments, constructs the adapters, injects
//! them into the use case, and maps domain errors to exit codes. No business
//! logic lives here — every decision this file makes is about wiring or about
//! how to talk to a terminal.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

mod cli;
mod exit;
mod presenter;

use std::process::ExitCode;

use adocpdf_adapters::dto::{from_domain_report, to_domain_request};
use adocpdf_asciidoc::parser::AsciidocParser;
use adocpdf_domain::ports::{Clock, Date};
use adocpdf_domain::render_document::RenderDocument;
use adocpdf_host::clock::{FixedClock, SystemClock};
use adocpdf_host::path_resolver::FilesystemPathResolver;
use adocpdf_host::source_store::FilesystemSourceStore;
use adocpdf_typst::renderer::TypstRenderer;
use adocpdf_typst::themes::BuiltInThemes;
use clap::Parser as _;

use crate::cli::Cli;
use crate::exit::code_for;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let clock: Box<dyn Clock> = match cli.date.as_deref().map(parse_date).transpose() {
        Ok(Some(date)) => Box::new(FixedClock::new(date)),
        Ok(None) => Box::new(SystemClock::new()),
        Err(message) => {
            eprintln!("adocpdf: {message}");
            return ExitCode::from(exit::USAGE);
        }
    };

    let resolver = FilesystemPathResolver::new();
    let sources = FilesystemSourceStore::new();
    let parser = AsciidocParser::new();
    let themes = BuiltInThemes::new();
    let renderer = TypstRenderer::new();

    let use_case = RenderDocument::new(
        &resolver,
        &sources,
        &parser,
        &themes,
        &renderer,
        clock.as_ref(),
    );

    match use_case.execute(&to_domain_request(&cli.to_request())) {
        Ok(report) => {
            // The use case's result crosses the boundary before it is spoken
            // aloud: `main` converts, and the presenter sees only the DTO.
            presenter::report_success(&from_domain_report(&report));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("adocpdf: {error}");
            ExitCode::from(code_for(&error))
        }
    }
}

/// Reads a `YYYY-MM-DD` date.
///
/// Hand-parsed rather than pulling in a date crate: the format is fixed, the
/// whole grammar is ten digits and two dashes, and the CLI layer is the wrong
/// place to grow a dependency.
fn parse_date(text: &str) -> Result<Date, String> {
    let mut parts = text.split('-');

    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(format!("--date must look like YYYY-MM-DD, got {text:?}"));
    };

    let year = year
        .parse::<i32>()
        .map_err(|_| format!("--date has an unreadable year: {year:?}"))?;
    let month = month
        .parse::<u8>()
        .map_err(|_| format!("--date has an unreadable month: {month:?}"))?;
    let day = day
        .parse::<u8>()
        .map_err(|_| format!("--date has an unreadable day: {day:?}"))?;

    Date::new(year, month, day).map_err(|error| format!("--date is not a date: {error}"))
}

/// Whether an error is worth suggesting the sandbox as the cause.
#[cfg(test)]
fn is_confinement(error: &adocpdf_domain::error::DomainError) -> bool {
    use adocpdf_domain::error::DomainError;

    matches!(
        error,
        DomainError::PathOutsideRoot { .. }
            | DomainError::ReferenceOutsideRoot { .. }
            | DomainError::RootNotADirectory { .. }
    )
}

#[cfg(test)]
mod tests {
    use adocpdf_domain::error::DomainError;

    use super::*;

    #[test]
    fn a_well_formed_date_is_accepted() {
        assert_eq!(
            parse_date("2026-08-16"),
            Ok(Date::new(2026, 8, 16).unwrap())
        );
    }

    #[test]
    fn a_date_with_too_few_parts_is_rejected() {
        assert!(parse_date("2026-08").is_err());
    }

    #[test]
    fn a_date_with_too_many_parts_is_rejected() {
        assert!(parse_date("2026-08-16-01").is_err());
    }

    #[test]
    fn a_date_with_a_non_numeric_part_is_rejected() {
        let error = parse_date("2026-aug-16").expect_err("months are numbers here");

        assert!(error.contains("month"), "got: {error}");
    }

    #[test]
    fn an_impossible_date_is_rejected() {
        assert!(parse_date("2026-13-01").is_err());
        assert!(parse_date("2026-01-32").is_err());
    }

    #[test]
    fn confinement_failures_are_recognisable() {
        assert!(is_confinement(&DomainError::PathOutsideRoot {
            requested: "x".to_owned(),
            root: "/p".to_owned(),
        }));
        assert!(!is_confinement(&DomainError::InputNotFound {
            path: "x".to_owned(),
        }));
    }
}
