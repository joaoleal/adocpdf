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
            Block::Paragraph(paragraph) => Some(paragraph.text().plain_text()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_document_title_becomes_the_document_title() {
    let outcome = parse("= The Report\n\nOpening words.\n");

    assert_eq!(
        outcome.document.title().map(InlineText::plain_text),
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
    assert_eq!(overview.heading().plain_text(), "Overview");
    assert_eq!(overview.level().get(), 1);

    let nested = overview
        .body()
        .iter()
        .find_map(|block| match block {
            Block::Section(section) => Some(section),
            _ => None,
        })
        .expect("the subsection must nest inside the section");
    assert_eq!(nested.heading().plain_text(), "Details");
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
    // A table, not a list: lists became supported, so the example moved.
    //
    // The cells are deliberately absent, and that is the requirement rather
    // than an accident of the example moving. An unsupported *inline*
    // construct keeps its text — see `the_text_an_unsupported_construct_carried
    // _is_kept` — because it sits inside a sentence that would otherwise lose a
    // phrase. A block does not: `one` and `two` poured into the paragraph
    // stream would read as prose the author never wrote, in an order they never
    // chose, with nothing left to say it had been a table.
    let outcome = parse("= Title\n\nBefore.\n\n|===\n| one | two\n|===\n\nAfter.\n");

    assert!(
        !outcome.skipped.is_empty(),
        "a table is not supported yet and must be reported"
    );
    assert_eq!(
        paragraphs(outcome.document.body()),
        ["Before.", "After."],
        "the rest of the document must still render, and the cells must not be \
         re-flowed into it"
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

// Inline structure, which the adapter used to discard by taking the source
// span for paragraphs and rendered HTML for headings.

use adocpdf_core::document::{InlineNode, InlineStyle, InlineText};

/// The styles applied anywhere within a run of inline content.
fn styles_in(text: &InlineText) -> Vec<InlineStyle> {
    fn walk(nodes: &[InlineNode], found: &mut Vec<InlineStyle>) {
        for node in nodes {
            if let InlineNode::Styled { style, children } = node {
                found.push(*style);
                walk(children, found);
            }
        }
    }
    let mut found = Vec::new();
    walk(text.nodes(), &mut found);
    found
}

#[test]
fn a_bold_word_in_a_paragraph_becomes_a_styled_span() {
    let outcome = parse("A bold *word* here.\n");
    let Block::Paragraph(paragraph) = &outcome.document.body()[0] else {
        panic!("expected a paragraph");
    };

    assert_eq!(paragraph.text().plain_text(), "A bold word here.");
    assert_eq!(styles_in(paragraph.text()), [InlineStyle::Strong]);
}

#[test]
fn a_formatted_document_title_carries_its_formatting_and_no_markup() {
    // The defect this change exists to correct: `doctitle()` returns rendered
    // output, so with the built-in HTML renderer this title reached the page
    // as `<strong>Bold</strong> Title`.
    let outcome = parse("= *Bold* Title\n\nBody.\n");
    let title = outcome.document.title().expect("the document has a title");

    assert_eq!(title.plain_text(), "Bold Title");
    assert_eq!(styles_in(title), [InlineStyle::Strong]);
    assert!(
        !title.plain_text().contains('<'),
        "no markup may reach the title, got {:?}",
        title.plain_text()
    );
}

#[test]
fn a_heading_and_a_paragraph_agree_on_the_same_inline_source() {
    // The two paths used to disagree: one took the source span, the other took
    // rendered HTML. Given the same inline source they must now produce the
    // same structure.
    let outcome = parse("== A *bold* heading\n\nA *bold* heading\n");
    let Block::Section(section) = &outcome.document.body()[0] else {
        panic!("expected a section");
    };
    let Block::Paragraph(paragraph) = &section.body()[0] else {
        panic!("expected a paragraph inside the section");
    };

    assert_eq!(section.heading().nodes(), paragraph.text().nodes());
}

#[test]
fn every_style_survives_the_adapter() {
    for (source, expected) in [
        ("*a*", InlineStyle::Strong),
        ("_a_", InlineStyle::Emphasis),
        ("`a`", InlineStyle::Monospace),
        ("^a^", InlineStyle::Superscript),
        ("~a~", InlineStyle::Subscript),
        ("#a#", InlineStyle::Highlight),
    ] {
        let outcome = parse(&format!("text {source} text\n"));
        let Block::Paragraph(paragraph) = &outcome.document.body()[0] else {
            panic!("expected a paragraph for {source}");
        };

        assert_eq!(styles_in(paragraph.text()), [expected], "for {source}");
    }
}

#[test]
fn an_attribute_reference_resolves_in_body_text() {
    let outcome = parse(":product: adocpdf\n\nBuilt with {product}.\n");

    assert_eq!(paragraphs(outcome.document.body()), ["Built with adocpdf."]);
}

#[test]
fn unsupported_inline_constructs_are_reported_with_their_block() {
    let outcome = parse(
        "See https://example.com[the site] and image:x.png[a picture].\n\n\
         Then a footnote:[note] on the second paragraph.\n",
    );

    let reported: Vec<&str> = outcome
        .skipped
        .iter()
        .map(|skip| skip.construct.as_str())
        .collect();
    assert_eq!(reported, ["link", "inline image", "footnote"]);

    // Each is attributed to the block it sat in, which is the finest
    // granularity the upstream API allows.
    assert_eq!(outcome.skipped[0].location.line, 1);
    assert_eq!(outcome.skipped[1].location.line, 1);
    assert_eq!(outcome.skipped[2].location.line, 3);
}

#[test]
fn no_markup_from_an_unsupported_construct_reaches_the_text() {
    let outcome = parse(
        ":experimental:\n\nA https://example.com[link], an image:x.png[image], \
         a footnote:[note], a kbd:[Ctrl+C] and a <<ref,cross-reference>>.\n",
    );

    for paragraph in paragraphs(outcome.document.body()) {
        assert!(
            !paragraph.contains('<') && !paragraph.contains("href"),
            "markup reached the page: {paragraph:?}"
        );
    }
}

#[test]
fn the_text_an_unsupported_construct_carried_is_kept() {
    let outcome = parse("See https://example.com[the site].\n");

    assert_eq!(paragraphs(outcome.document.body()), ["See the site."]);
}

#[test]
fn an_undefined_attribute_reference_is_reported_and_left_as_written() {
    let outcome = parse("Built with {nowhere-defined}.\n");

    assert_eq!(
        paragraphs(outcome.document.body()),
        ["Built with {nowhere-defined}."],
        "the reference stays as the author wrote it rather than becoming empty text"
    );

    let reported: Vec<&str> = outcome
        .skipped
        .iter()
        .map(|skip| skip.construct.as_str())
        .collect();
    assert_eq!(reported.len(), 1, "expected one report, got {reported:?}");
    assert!(
        reported[0].contains("nowhere-defined"),
        "the report must name the attribute, got {reported:?}"
    );
}

#[test]
fn a_defined_attribute_is_substituted_and_not_reported() {
    let outcome = parse(":product: adocpdf\n\nBuilt with {product}.\n");

    assert_eq!(paragraphs(outcome.document.body()), ["Built with adocpdf."]);
    assert!(
        outcome.skipped.is_empty(),
        "a resolved reference is not a skipped construct, got {:?}",
        outcome.skipped
    );
}
