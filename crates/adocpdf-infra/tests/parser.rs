//! The parser adapter against real AsciiDoc sources.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use adocpdf_core::document::Block;
use adocpdf_domain::ports::{Date, DocumentParser, ParseOutcome};
use adocpdf_infra::parser::AsciidocParser;

fn parse(source: &str) -> ParseOutcome {
    AsciidocParser::new()
        .parse(source, "book.adoc", Date::new(2026, 8, 16).unwrap())
        .expect("the source parses")
}

fn paragraphs(blocks: &[Block]) -> Vec<String> {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(paragraph) => Some(paragraph.text().as_str().to_owned()),
            Block::Section(_) => None,
        })
        .collect()
}

#[test]
fn a_document_title_becomes_the_document_title() {
    let outcome = parse("= The Report\n\nOpening words.\n");

    assert_eq!(
        outcome.document.title().map(|t| t.as_str().to_owned()),
        Some("The Report".to_owned())
    );
}

#[test]
fn a_document_without_a_title_has_none() {
    let outcome = parse("Just a paragraph.\n");

    assert!(outcome.document.title().is_none());
}

#[test]
fn paragraphs_carry_their_text() {
    let outcome = parse("= Title\n\nFirst paragraph.\n\nSecond paragraph.\n");

    assert_eq!(
        paragraphs(outcome.document.body()),
        ["First paragraph.", "Second paragraph."]
    );
}

#[test]
fn sections_nest_and_keep_their_levels() {
    let outcome =
        parse("= Title\n\n== Overview\n\nOverview body.\n\n=== Details\n\nDetail body.\n");

    let Block::Section(overview) = &outcome.document.body()[0] else {
        panic!("expected a section, got {:?}", outcome.document.body());
    };
    assert_eq!(overview.heading().as_str(), "Overview");
    assert_eq!(overview.level().get(), 1);

    let nested = overview
        .body()
        .iter()
        .find_map(|block| match block {
            Block::Section(section) => Some(section),
            Block::Paragraph(_) => None,
        })
        .expect("the subsection must nest inside the section");
    assert_eq!(nested.heading().as_str(), "Details");
    assert_eq!(nested.level().get(), 2);
}

#[test]
fn a_section_can_declare_a_theme() {
    let outcome = parse("= Title\n\n[theme=wide]\n== Appendix\n\nBody.\n");

    let Block::Section(section) = &outcome.document.body()[0] else {
        panic!("expected a section");
    };
    assert_eq!(
        section.theme().map(adocpdf_core::ThemeId::as_str),
        Some("wide")
    );
}

#[test]
fn a_section_without_a_theme_declares_none() {
    let outcome = parse("= Title\n\n== Plain\n\nBody.\n");

    let Block::Section(section) = &outcome.document.body()[0] else {
        panic!("expected a section");
    };
    assert!(section.theme().is_none());
}

#[test]
fn a_malformed_theme_name_is_reported_rather_than_passed_on() {
    let outcome = parse("= Title\n\n[theme=NotKebab]\n== Appendix\n\nBody.\n");

    let Block::Section(section) = &outcome.document.body()[0] else {
        panic!("expected a section");
    };
    assert!(
        section.theme().is_none(),
        "an unusable name must not travel onward as if it were valid"
    );
    assert!(
        outcome
            .skipped
            .iter()
            .any(|skipped| skipped.construct.contains("theme")),
        "the author must be told, got: {:?}",
        outcome.skipped
    );
}

#[test]
fn an_unsupported_construct_is_skipped_rather_than_fatal() {
    let outcome = parse("= Title\n\nBefore.\n\n* one\n* two\n\nAfter.\n");

    assert!(
        !outcome.skipped.is_empty(),
        "a list is not supported yet and must be reported"
    );
    assert_eq!(
        paragraphs(outcome.document.body()),
        ["Before.", "After."],
        "the rest of the document must still render"
    );
}

#[test]
fn a_skipped_construct_reports_where_it_was() {
    let outcome = parse("= Title\n\nBefore.\n\n|===\n| a | b\n|===\n");

    let skipped = outcome
        .skipped
        .first()
        .expect("the table must be reported as skipped");

    assert!(
        skipped.location.line > 1,
        "the location must point into the source, got {:?}",
        skipped.location
    );
}

#[test]
fn a_document_using_only_supported_constructs_skips_nothing() {
    let outcome = parse("= Title\n\nBody.\n\n== Section\n\nMore body.\n");

    assert!(
        outcome.skipped.is_empty(),
        "nothing here is unsupported, got: {:?}",
        outcome.skipped
    );
}

#[test]
fn parsing_the_same_source_twice_gives_the_same_document() {
    let source = "= Title\n\nBody.\n\n== Section\n\nMore.\n";

    assert_eq!(
        parse(source).document,
        parse(source).document,
        "parsing must be reproducible for output to be byte-identical"
    );
}

#[test]
fn an_include_directive_does_not_read_the_file() {
    // Safe mode is the reason: a document must not be able to widen its own
    // access. The include must not appear as resolved content.
    let outcome = parse("= Title\n\ninclude::/etc/passwd[]\n");

    let text = paragraphs(outcome.document.body()).join("\n");
    assert!(
        !text.contains("root:"),
        "no file content may leak into the document, got: {text}"
    );
}

#[test]
fn input_that_looks_malformed_still_produces_a_document() {
    // AsciiDoc has no parse errors: every byte sequence is a valid document,
    // and the upstream parser is total. This is a property of the language, not
    // a gap in the adapter — see the note in the change's design.
    let outcome = AsciidocParser::new()
        .parse(
            "=== \n\n[[[\n\n****\nunclosed\n",
            "broken.adoc",
            Date::new(2026, 8, 16).unwrap(),
        )
        .expect("parsing must return a document rather than an error");

    assert!(
        outcome.document.title().is_none(),
        "the mangled heading is not a document title"
    );
}
