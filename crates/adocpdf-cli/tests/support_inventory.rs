//! `docs/asciidoc-support.md`, checked against the renderer it describes.
//!
//! The inventory is the scope of every later change, so a row that claims more
//! than the code does is worse than no inventory at all. These tests read the
//! file and hold it to what it says:
//!
//! - a row marked `honoured` must render its sample with nothing reported as
//!   skipped;
//! - a row marked `partial` must name a later tier, so a partial row cannot sit
//!   there indefinitely without a commitment to finish it;
//! - a row marked `never` must give a reason.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::PathBuf;

use adocpdf_asciidoc::parser::AsciidocParser;
use adocpdf_core::theme::ThemeSet;
use adocpdf_domain::document_plan::plan_document;
use adocpdf_domain::ports::{Date, DocumentParser, DocumentRenderer};
use adocpdf_typst::renderer::TypstRenderer;

/// One row of the inventory.
#[derive(Debug)]
struct Row {
    construct: String,
    status: String,
    tier: String,
    sample: Option<String>,
}

fn inventory_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("docs/asciidoc-support.md")
}

/// Reads every construct row, skipping the legend and the header rows.
fn rows() -> Vec<Row> {
    let text = fs::read_to_string(inventory_path()).expect("the inventory is present");

    text.lines()
        .filter(|line| line.starts_with('|'))
        .filter_map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();

            // Construct | Syntax | Parser | Status | Tier | Sample
            if cells.len() != 6 {
                return None;
            }
            // The header row and its underline.
            if cells[0] == "Construct" || cells[0].starts_with("---") {
                return None;
            }

            let sample = cells[5].trim_matches('`');
            Some(Row {
                construct: cells[0].trim_matches('*').to_owned(),
                // The inventory emboldens the rows that will never be
                // supported, so that a reader skimming it sees them.
                status: cells[3].trim_matches('*').to_owned(),
                tier: cells[4].to_owned(),
                sample: (cells[5] != "—").then(|| unescape(sample)),
            })
        })
        .collect()
}

/// Turns the inventory's `\n` into a real line break.
///
/// Markdown table cells cannot hold a newline, so samples spell one out. The
/// alternative — a separate fixture file per row — would put the sample
/// somewhere other than the claim it supports, which is exactly how the two
/// drift apart.
fn unescape(sample: &str) -> String {
    sample.replace("\\n", "\n").trim_matches('"').to_owned()
}

/// Renders a sample and returns what was reported as skipped.
fn skipped_by(sample: &str) -> Vec<String> {
    let outcome = AsciidocParser::new()
        .parse(sample, "inventory.adoc", Date::new(2026, 8, 16).unwrap())
        .unwrap_or_else(|error| panic!("the sample {sample:?} must parse: {error}"));

    let plan = plan_document(&outcome.document, &ThemeSet::default())
        .unwrap_or_else(|error| panic!("the sample {sample:?} must plan: {error}"));

    TypstRenderer::new()
        .render(&plan, "inventory.adoc", Date::new(2026, 8, 16).unwrap())
        .unwrap_or_else(|error| panic!("the sample {sample:?} must render: {error}"));

    outcome
        .skipped
        .into_iter()
        .map(|skip| skip.construct)
        .collect()
}

#[test]
fn the_inventory_is_readable_and_not_empty() {
    let rows = rows();

    assert!(
        rows.len() > 80,
        "the inventory should cover the language, got {} rows",
        rows.len()
    );
}

#[test]
fn every_status_is_one_of_the_four() {
    for row in rows() {
        assert!(
            ["honoured", "scheduled", "never"].contains(&row.status.as_str())
                || row.status.starts_with("partial"),
            "{:?} has the unknown status {:?}",
            row.construct,
            row.status
        );
    }
}

#[test]
fn every_honoured_row_renders_its_sample_without_skipping_anything() {
    let honoured: Vec<Row> = rows()
        .into_iter()
        .filter(|row| row.status == "honoured")
        .collect();

    // Without this the test would pass by parsing nothing at all, which is the
    // one way a check on a document can quietly stop checking.
    assert!(
        honoured.len() > 40,
        "only {} honoured rows were read; the table format has probably changed",
        honoured.len()
    );

    for row in honoured {
        let sample = row.sample.as_ref().unwrap_or_else(|| {
            panic!(
                "{:?} claims to be honoured but gives no sample",
                row.construct
            )
        });

        let skipped = skipped_by(sample);

        assert!(
            skipped.is_empty(),
            "{:?} is marked honoured, but rendering its sample reported {skipped:?}",
            row.construct
        );
    }
}

#[test]
fn every_partial_row_names_a_later_tier() {
    for row in rows()
        .into_iter()
        .filter(|row| row.status.starts_with("partial"))
    {
        assert!(
            row.status.contains("tier"),
            "{:?} is partial without saying which tier finishes it",
            row.construct
        );
        assert!(
            row.sample.is_some(),
            "{:?} is partial and must still show what it does render",
            row.construct
        );
        assert_eq!(
            row.tier, "1",
            "a partial row is one tier 1 started; got tier {:?} for {:?}",
            row.tier, row.construct
        );
    }
}

#[test]
fn every_partial_row_still_renders() {
    // Partial means incomplete, not broken: whatever it does render must work.
    for row in rows()
        .into_iter()
        .filter(|row| row.status.starts_with("partial"))
    {
        let sample = row.sample.expect("a partial row has a sample");
        drop(skipped_by(&sample));
    }
}

#[test]
fn a_scheduled_row_names_the_tier_that_will_honour_it() {
    for row in rows().into_iter().filter(|row| row.status == "scheduled") {
        let tier: u8 = row.tier.parse().unwrap_or_else(|_| {
            panic!(
                "{:?} is scheduled for the unparsable tier {:?}",
                row.construct, row.tier
            )
        });

        assert!(
            (2..=5).contains(&tier),
            "{:?} is scheduled for tier {tier}, which is not a later tier",
            row.construct
        );
    }
}

#[test]
fn a_never_row_has_no_tier_and_a_reason_in_the_prose() {
    let text = fs::read_to_string(inventory_path()).expect("the inventory is present");

    for row in rows()
        .into_iter()
        .filter(|row| row.status.contains("never"))
    {
        assert_eq!(
            row.tier, "—",
            "{:?} is never supported, so it cannot have a tier",
            row.construct
        );
        assert!(
            text.contains("never supported"),
            "the inventory must say why {:?} will never be supported",
            row.construct
        );
    }
}

#[test]
fn the_never_supported_set_is_exactly_what_the_specs_say() {
    let never: Vec<String> = rows()
        .into_iter()
        .filter(|row| row.status.contains("never"))
        .map(|row| row.construct.to_lowercase())
        .collect();

    assert_eq!(
        never,
        ["audio", "video", "docinfo"],
        "adding to this set is a decision, not an edit"
    );
}
