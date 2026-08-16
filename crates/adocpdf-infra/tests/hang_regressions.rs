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
