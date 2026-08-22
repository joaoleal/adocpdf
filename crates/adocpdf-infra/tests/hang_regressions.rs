//! Inputs that fuzzing found, kept as ordinary tests.
//!
//! The fuzzer needs nightly and runs on a schedule; these run on the pinned
//! stable toolchain in every `cargo test`. That is the arrangement the change
//! this file came from describes: the fuzzer is the finder, the stable suite is
//! the record, and a contributor who never installs nightly still runs every
//! bug fuzzing has ever found.
//!
//! # Why these are written with a timeout rather than as plain calls
//!
//! The finding here is a **hang**, not a panic. A test that simply called the
//! offending path would not fail — it would never return, and the gate would
//! sit there until CI killed the job, reporting a timeout instead of a named
//! failing test. So each case runs on its own thread and the test asserts the
//! work finished inside a budget. A hang then shows up as a normal, named test
//! failure with a useful message.
//!
//! The budget is deliberately loose. It is not a performance assertion — it is
//! the difference between "returns" and "does not return", and it should not
//! start failing on a slow machine.
//!
//! Not everything fuzzing found lives here. The same target later found a
//! *panic* — an inline `image:` macro with no target — and a panic already
//! fails a test without any of this machinery, so that reproducer is recorded
//! in `tests/parser_refusal.rs` beside the guard that catches it.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use adocpdf_core::theme::ThemeSet;
use adocpdf_domain::document_plan::plan_document;
use adocpdf_domain::ports::{Date, DocumentParser};
use adocpdf_infra::emitter::emit;
use adocpdf_infra::parser::AsciidocParser;

/// Generous on purpose: see the module docs. The work under test either
/// returns in milliseconds or does not return at all.
const BUDGET: Duration = Duration::from_secs(20);

/// Runs `work` on its own thread and reports whether it finished in time.
///
/// The thread is deliberately leaked when it does not: there is no way to kill
/// a stuck thread in safe Rust, and the test process is about to fail and exit
/// anyway.
fn finishes_within<F>(work: F) -> bool
where
    F: FnOnce() + Send + 'static,
{
    let (done, finished) = mpsc::channel();

    thread::spawn(move || {
        work();
        // The receiver is gone if the budget already expired; that is fine and
        // is not something this thread can do anything about.
        let _ = done.send(());
    });

    finished.recv_timeout(BUDGET).is_ok()
}

fn a_date() -> Date {
    Date::new(2026, 8, 16).expect("a real date")
}

/// A single form feed, found by the `parse_plan_emit` fuzz target within
/// seconds of starting, minimised by libFuzzer to one byte.
///
/// Recorded as `fuzz/artifacts/parse_plan_emit/timeout-1e32e3c3...`.
const FORM_FEED: &str = "\u{c}";

#[test]
fn a_lone_form_feed_does_not_hang_the_parser() {
    assert!(
        finishes_within(|| {
            drop(AsciidocParser.parse(FORM_FEED, "fuzz.adoc", a_date()));
        }),
        "parsing a single form feed (U+000C) did not finish within {BUDGET:?}. \
         This is the fuzzing finding recorded in the behavioural-testing change; \
         if it is still failing, the defect is not yet fixed."
    );
}

#[test]
fn a_lone_form_feed_does_not_hang_the_whole_pure_path() {
    assert!(
        finishes_within(|| {
            let Ok(outcome) = AsciidocParser.parse(FORM_FEED, "fuzz.adoc", a_date()) else {
                return;
            };
            let Ok(plan) = plan_document(&outcome.document, &ThemeSet::default()) else {
                return;
            };
            drop(emit(&plan));
        }),
        "parse -> plan -> emit on a single form feed (U+000C) did not finish \
         within {BUDGET:?}"
    );
}

/// The same byte embedded in ordinary text, to show whether the problem is the
/// character itself or the document being nothing but that character.
#[test]
fn a_form_feed_between_words_does_not_hang_the_parser() {
    let source = format!("Hello{FORM_FEED}world");

    assert!(
        finishes_within(move || {
            drop(AsciidocParser.parse(&source, "fuzz.adoc", a_date()));
        }),
        "parsing text containing a form feed did not finish within {BUDGET:?}"
    );
}

/// A line of carriage returns, found by the same fuzz target and minimised by
/// libFuzzer to four bytes.
///
/// Recorded as `fuzz/artifacts/parse_plan_emit/timeout-8a6170bc83e3...`.
const CARRIAGE_RETURN_LINE: &str = "\n\r\r\r";

/// The same defect reached through a document that has real content in it.
///
/// Worth its own case: the earlier form-feed hang only ever affected documents
/// with nothing in them, so a guard shaped around that assumption would let
/// this one through. This is a title, a blank line, and a line of carriage
/// returns — an ordinary document, as far as an author is concerned.
const TITLED_DOCUMENT_WITH_CARRIAGE_RETURNS: &str = "= T\n\n\r\r";

#[test]
fn a_line_of_carriage_returns_does_not_hang_the_parser() {
    assert!(
        finishes_within(|| {
            drop(AsciidocParser.parse(CARRIAGE_RETURN_LINE, "fuzz.adoc", a_date()));
        }),
        "parsing a line of carriage returns did not finish within {BUDGET:?}. \
         asciidoc-parser 0.29.19 does not terminate on it; if this is failing, \
         the guard that refuses it has been removed or weakened."
    );
}

#[test]
fn a_titled_document_with_carriage_returns_does_not_hang_the_parser() {
    assert!(
        finishes_within(|| {
            drop(AsciidocParser.parse(
                TITLED_DOCUMENT_WITH_CARRIAGE_RETURNS,
                "fuzz.adoc",
                a_date(),
            ));
        }),
        "parsing a titled document ending in a line of carriage returns did not \
         finish within {BUDGET:?}"
    );
}

#[test]
fn a_line_of_carriage_returns_does_not_hang_the_whole_pure_path() {
    assert!(
        finishes_within(|| {
            let Ok(outcome) = AsciidocParser.parse(CARRIAGE_RETURN_LINE, "fuzz.adoc", a_date())
            else {
                return;
            };
            let Ok(plan) = plan_document(&outcome.document, &ThemeSet::default()) else {
                return;
            };
            drop(emit(&plan));
        }),
        "parse -> plan -> emit on a line of carriage returns did not finish \
         within {BUDGET:?}"
    );
}

/// A vertical tab alone on a line of a document that has content, found by the
/// same fuzz target on a later run and minimised to five bytes.
///
/// Recorded as `fuzz/artifacts/parse_plan_emit/timeout-f4c7a0722eaa...`.
///
/// This is the third finding of the family, and the one that showed the first
/// guard had been written against the examples rather than the defect: a
/// vertical tab hangs the parser wherever it sits alone on a line, not only in
/// a document that holds nothing else.
const VERTICAL_TAB_ALONE_ON_A_LINE: &str = "[;;\n\u{b}";

#[test]
fn a_vertical_tab_alone_on_a_line_does_not_hang_the_parser() {
    assert!(
        finishes_within(|| {
            drop(AsciidocParser.parse(VERTICAL_TAB_ALONE_ON_A_LINE, "fuzz.adoc", a_date()));
        }),
        "parsing a vertical tab alone on a line of a document with content did \
         not finish within {BUDGET:?}"
    );
}

#[test]
fn a_vertical_tab_alone_on_a_line_does_not_hang_the_whole_pure_path() {
    assert!(
        finishes_within(|| {
            let Ok(outcome) =
                AsciidocParser.parse(VERTICAL_TAB_ALONE_ON_A_LINE, "fuzz.adoc", a_date())
            else {
                return;
            };
            let Ok(plan) = plan_document(&outcome.document, &ThemeSet::default()) else {
                return;
            };
            drop(emit(&plan));
        }),
        "parse -> plan -> emit on a vertical tab alone on a line did not finish \
         within {BUDGET:?}"
    );
}

/// A form feed sharing its line with content, found by the fuzz target after
/// the guard had been narrowed to lines holding nothing else.
///
/// Recorded as `fuzz/artifacts/parse_plan_emit/timeout-2ceb781d9de7...`.
///
/// The fourth finding of the family, and the one that settled the argument:
/// three successively wider rules about *where* the character sat were each
/// outflanked by the next fuzzing run, so the guard is now about the character
/// itself.
const FORM_FEED_BESIDE_CONTENT: &str = ";toc::  \u{c}";

#[test]
fn a_form_feed_beside_content_does_not_hang_the_parser() {
    assert!(
        finishes_within(|| {
            drop(AsciidocParser.parse(FORM_FEED_BESIDE_CONTENT, "fuzz.adoc", a_date()));
        }),
        "parsing a form feed sharing its line with content did not finish \
         within {BUDGET:?}"
    );
}

#[test]
fn a_form_feed_beside_content_does_not_hang_the_whole_pure_path() {
    assert!(
        finishes_within(|| {
            let Ok(outcome) = AsciidocParser.parse(FORM_FEED_BESIDE_CONTENT, "fuzz.adoc", a_date())
            else {
                return;
            };
            let Ok(plan) = plan_document(&outcome.document, &ThemeSet::default()) else {
                return;
            };
            drop(emit(&plan));
        }),
        "parse -> plan -> emit on a form feed beside content did not finish \
         within {BUDGET:?}"
    );
}
