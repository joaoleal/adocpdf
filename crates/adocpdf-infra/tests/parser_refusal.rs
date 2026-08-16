//! The parse boundary's refusal, from both sides.
//!
//! `tests/hang_regressions.rs` records the inputs fuzzing found and asserts
//! they terminate. This file describes the guard that makes them terminate:
//! what it refuses, and — the part worth testing — what it deliberately does
//! not.
//!
//! The distinction is the whole design. `asciidoc-parser` 0.29.19 hangs only
//! on a document that contains a vertical tab or form feed and no other
//! content. Refusing every occurrence of those characters would have been
//! simpler and would have broken `"Hello\u{c}world"`, which parses today.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use adocpdf_domain::error::DomainError;
use adocpdf_domain::ports::{Date, DocumentParser, ParseOutcome};
use adocpdf_infra::parser::AsciidocParser;

fn parse(source: &str) -> Result<ParseOutcome, DomainError> {
    let today = Date::new(2026, 8, 16).expect("a real date");
    AsciidocParser.parse(source, "boundary.adoc", today)
}

#[test]
fn a_whitespace_only_document_containing_a_form_feed_is_refused() {
    let error = parse("\u{c}").expect_err("this input cannot be parsed without hanging");

    assert!(
        matches!(error, DomainError::ParseFailed { .. }),
        "expected a parse failure, got {error:?}"
    );
    assert!(
        error.to_string().contains("U+000C"),
        "the error must name the character, got: {error}"
    );
}

#[test]
fn a_whitespace_only_document_containing_a_vertical_tab_is_refused() {
    let error = parse("  \u{b}\n").expect_err("this input cannot be parsed without hanging");

    assert!(
        error.to_string().contains("U+000B"),
        "the error must name the character, got: {error}"
    );
}

#[test]
fn the_same_characters_beside_real_content_are_accepted() {
    for source in [
        "Hello\u{c}world",
        "a\u{c}",
        "\u{c}a",
        "a\n\u{c}",
        "\u{b}text",
    ] {
        assert!(
            parse(source).is_ok(),
            "{source:?} parses today and must continue to"
        );
    }
}

#[test]
fn ordinary_empty_documents_are_still_accepted() {
    for source in ["", "   ", "\n", "\n\n  \t "] {
        assert!(
            parse(source).is_ok(),
            "{source:?} contains no offending character and must be accepted"
        );
    }
}

#[test]
fn the_refusal_names_the_document_it_refused() {
    let error = parse("\u{c}").expect_err("refused");

    assert!(
        error.to_string().contains("boundary.adoc"),
        "the error must name the document, got: {error}"
    );
}

#[test]
fn the_refusal_is_distinguishable_from_a_syntax_error() {
    let error = parse("\u{c}").expect_err("refused");
    let message = error.to_string();

    // An author reading this needs to know the document was declined because
    // of a defect in the parser, not because they wrote something invalid.
    assert!(
        message.contains("asciidoc-parser"),
        "the message must say what could not handle it, got: {message}"
    );
    assert!(
        message.contains("does not terminate"),
        "the message must say why it was declined, got: {message}"
    );
}
