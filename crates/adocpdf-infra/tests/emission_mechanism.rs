//! Does the engine accept the shape the emitter intends to produce?
//!
//! `design.md` D4 assumes a paragraph can be emitted entirely in code mode by
//! concatenating content values — `#par(text("a") + strong(text("b")))` —
//! rather than by switching to markup mode. Everything downstream depends on
//! it: emitting in markup mode instead would put the escaping argument back on
//! the table, because markup is whitespace- and position-sensitive.
//!
//! The assumption was recorded as unverified. This file settles it against the
//! real engine, and it inspects the laid-out frame rather than the extracted
//! text, because extraction reports characters and the question here is
//! whether a *face* changed.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use adocpdf_domain::ports::Date;
use adocpdf_infra::fonts::{BODY_FAMILY, EmbeddedFonts};
use adocpdf_infra::world::InMemoryWorld;
use typst::layout::FrameItem;
use typst::text::{FontWeight, TextItem};
use typst_layout::PagedDocument;

/// Lays markup out and returns the text runs the engine produced.
fn text_runs(markup: &str) -> Vec<(String, FontWeight)> {
    let world = InMemoryWorld::new(
        markup.to_owned(),
        EmbeddedFonts::load(),
        Date::new(2026, 8, 20).unwrap(),
    );

    let document = typst::compile::<PagedDocument>(&world)
        .output
        .expect("the markup compiles");

    let mut runs = Vec::new();
    for page in document.pages() {
        collect(&page.frame, &mut runs);
    }
    runs
}

fn collect(frame: &typst::layout::Frame, runs: &mut Vec<(String, FontWeight)>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Text(TextItem { text, font, .. }) => {
                runs.push((text.to_string(), font.info().variant.weight));
            }
            FrameItem::Group(group) => collect(&group.frame, runs),
            _ => {}
        }
    }
}

/// The markup the emitter would produce for `Hello *world*.`, in code mode.
fn concatenated_paragraph() -> String {
    format!(
        "#set text(font: \"{BODY_FAMILY}\", size: 11pt)\n\
         #par(text(\"a\") + strong(text(\"b\")))\n"
    )
}

#[test]
fn content_values_concatenate_in_code_mode() {
    let runs = text_runs(&concatenated_paragraph());

    let seen: String = runs.iter().map(|(text, _)| text.as_str()).collect();

    assert_eq!(
        seen, "ab",
        "the engine must accept a paragraph built by concatenating content values; \
         got the runs {runs:?}"
    );
}

#[test]
fn a_concatenated_strong_run_is_actually_bold() {
    let runs = text_runs(&concatenated_paragraph());

    let plain = runs
        .iter()
        .find(|(text, _)| text == "a")
        .expect("the plain run is present");
    let strong = runs
        .iter()
        .find(|(text, _)| text == "b")
        .expect("the strong run is present");

    assert_eq!(
        plain.1,
        FontWeight::REGULAR,
        "the plain run must stay plain"
    );
    assert_eq!(
        strong.1,
        FontWeight::BOLD,
        "the strong run must resolve to a bold face; Typst does not synthesise one, so a \
         regular weight here means the bold face is missing from the book rather than that \
         the markup was wrong"
    );
}

// The line breaker, and the settings that go with it.

use adocpdf_core::document::{Block, Document, InlineText, Paragraph};
use adocpdf_core::theme::ThemeSet;
use adocpdf_domain::document_plan::plan_document;
use adocpdf_infra::emitter::emit;

/// A paragraph that the two line breakers lay out differently.
///
/// Not any paragraph will do. On ordinary prose at this measure — A4 with 20mm
/// margins, set at 11pt, so roughly seventy-five characters to the line —
/// greedy and optimal breaking usually agree, and a test built on text where
/// they agree would pass whether or not the setting had any effect. This text
/// mixes very long words with very short ones, which is where a greedy pass
/// paints itself into a corner and an optimal one does not.
fn paragraph_that_distinguishes_the_breakers() -> String {
    let document = Document::new().with_block(Block::Paragraph(Paragraph::new(InlineText::new(
        "antidisestablishmentarianism and typesetting counterrevolutionaries a and \
         measure dog a typesetting incomprehensibilities dog layout \
         uncharacteristically is dog typesetting and is layout internationalization and \
         renderer extraordinarily a and layout dog a renderer notwithstanding dog engine \
         pneumonoultramicroscopicsilicovolcanoconiosis is dog renderer and is engine.",
    ))));
    let plan = plan_document(&document, &ThemeSet::default()).expect("a paragraph plans");
    emit(&plan)
}

#[test]
fn the_emitter_asks_for_optimal_line_breaking() {
    assert!(
        paragraph_that_distinguishes_the_breakers().contains(r#"linebreaks: "optimized""#),
        "the engine falls back to a greedy first-fit pass unless asked otherwise"
    );
}

#[test]
fn optimal_breaking_actually_changes_where_the_lines_break() {
    // Asserting the setting is present only proves we wrote it down. This
    // proves the engine acted on it: the same paragraph laid out with the
    // greedy breaker must produce different lines.
    let optimized = paragraph_that_distinguishes_the_breakers();
    let simple = optimized.replace(r#""optimized""#, r#""simple""#);

    let optimized_runs: Vec<String> = text_runs(&optimized)
        .into_iter()
        .map(|(text, _)| text)
        .collect();
    let simple_runs: Vec<String> = text_runs(&simple)
        .into_iter()
        .map(|(text, _)| text)
        .collect();

    assert_ne!(
        optimized_runs, simple_runs,
        "the two breakers produced identical lines, so the setting had no effect"
    );
    assert_eq!(
        optimized_runs.concat().replace(' ', ""),
        simple_runs.concat().replace(' ', ""),
        "the same words must be present either way; only the breaks differ"
    );
}

#[test]
fn the_emitter_leaves_text_ragged_right_by_default() {
    assert!(
        paragraph_that_distinguishes_the_breakers().contains("justify: false"),
        "justification is a separate decision from the choice of line breaker"
    );
}

#[test]
fn widow_and_orphan_avoidance_is_emitted() {
    assert!(
        paragraph_that_distinguishes_the_breakers().contains("costs: (orphan: 100%, widow: 100%)"),
        "the engine's widow and orphan prevention is driven by these costs"
    );
}

#[test]
fn no_language_is_named_unless_the_theme_names_one() {
    assert!(
        !paragraph_that_distinguishes_the_breakers().contains("lang:"),
        "a language nobody asked for would turn on hyphenation nobody asked for"
    );
}

/// The whole path: source through the parser, plan, emitter and engine.
fn render_source(source: &str) -> Vec<(String, FontWeight)> {
    use adocpdf_domain::ports::DocumentParser;
    use adocpdf_infra::parser::AsciidocParser;

    let outcome = AsciidocParser::new()
        .parse(source, "book.adoc", Date::new(2026, 8, 20).unwrap())
        .expect("the source parses");
    let plan = plan_document(&outcome.document, &ThemeSet::default()).expect("the document plans");

    text_runs(&emit(&plan))
}

#[test]
fn a_bold_word_in_source_is_set_in_a_bold_face() {
    let runs = render_source("An ordinary word and a *bold* one.\n");

    let bold: Vec<&String> = runs
        .iter()
        .filter(|(_, weight)| *weight == FontWeight::BOLD)
        .map(|(text, _)| text)
        .collect();

    assert_eq!(
        bold.len(),
        1,
        "exactly one run should be bold, got {runs:?}"
    );
    assert!(bold[0].contains("bold"), "the bold run is {:?}", bold[0]);
}

#[test]
fn the_delimiters_do_not_reach_the_page() {
    let runs = render_source("An ordinary word and a *bold* one.\n");
    let page: String = runs.iter().map(|(text, _)| text.as_str()).collect();

    assert!(
        !page.contains('*'),
        "the formatting delimiters must not be visible, got {page:?}"
    );
    assert!(page.contains("bold"));
}
