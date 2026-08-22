//! Tier 2 of the injection property: the engine as its own oracle.
//!
//! The Tier 1 properties beside `string_literal` prove the output matches
//! *our* model of an engine string literal. They would pass a systematic
//! misunderstanding of Typst's grammar just as happily as a correct one, since
//! the inverse decoder they round-trip through was written from the same
//! understanding.
//!
//! So this file asks the engine. It emits text through the real rendering path
//! the renderer uses, then reads the text back out of the finished PDF and
//! asserts it is what went in. That is expensive — a full layout and PDF
//! export per case — so the case count is low by design, and Tier 1 does the
//! exploring.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use adocpdf_core::document::{Block, Document, InlineText, Paragraph};
use adocpdf_core::theme::ThemeSet;
use adocpdf_domain::document_plan::plan_document;
use adocpdf_domain::ports::{Date, DocumentRenderer};
use adocpdf_infra::renderer::TypstRenderer;
use proptest::prelude::*;

/// Text that a PDF can actually represent and that extraction can return.
///
/// This is narrower than Tier 1's generator, deliberately. Tier 1 asks what
/// the escaper does with any input at all, including control characters that
/// have no visual form. Here the question is different — whether the engine
/// renders our literal as the characters we meant — and a character with no
/// glyph, or one the PDF text layer normalises, cannot answer it either way.
///
/// The characters that matter most are still present: the quote and the
/// backslash, which are what an injection would have to use, and the markup
/// metacharacters that would mean something if this were emitted as markup.
fn renderable_text() -> impl Strategy<Value = String> {
    let character = prop_oneof![
        6 => prop::char::range('a', 'z'),
        2 => prop::char::range('A', 'Z'),
        2 => prop::char::range('0', '9'),
        4 => prop_oneof![
            Just('"'), Just('\\'), Just('*'), Just('_'), Just('#'),
            Just('$'), Just('@'), Just('<'), Just('>'), Just('`'),
            Just('['), Just(']'), Just('('), Just(')'), Just('='),
            Just('-'), Just('+'), Just('/'), Just('~'), Just('^'),
        ],
        2 => Just(' '),
    ];

    proptest::collection::vec(character, 1..24).prop_map(|characters| {
        let text: String = characters.into_iter().collect();
        // Leading and trailing spaces do not survive a round trip through PDF
        // text extraction, and that is a property of PDF, not of the escaper.
        text.trim().to_owned()
    })
}

/// The text a reader would see, extracted from the finished PDF.
fn text_of(pdf: &[u8]) -> String {
    pdf_extract::extract_text_from_mem(pdf).expect("the PDF can be read back")
}

/// Renders one paragraph and returns what the PDF says it contains.
fn render_paragraph(text: &str) -> String {
    let document =
        Document::new().with_block(Block::Paragraph(Paragraph::new(InlineText::new(text))));
    let plan = plan_document(&document, &ThemeSet::default()).expect("a paragraph plans");
    let bytes = TypstRenderer::new()
        .render(&plan, "property.adoc", Date::new(2026, 8, 16).unwrap())
        .expect("a paragraph renders");

    text_of(&bytes)
}

proptest! {
    // A hundred cases, not the default 256, and not a guess: measured at
    // 13ms per case (20 cases in 0.26s, 200 in 2.63s — linear, since each one
    // is a full Typst layout plus a PDF export). A hundred costs about 1.3s.
    //
    // The number is a trade, and the trade is only this tier's. Tier 1 runs
    // the default case count against the same function for free, so exploring
    // the input space is its job; this tier exists to catch a disagreement
    // between our model of a string literal and the engine's, and a
    // disagreement that systematic does not need many cases to show up.
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Whatever the source said, that is what the reader sees — and nothing
    /// the source said became an instruction on the way.
    #[test]
    fn text_survives_the_real_rendering_path_unchanged(text in renderable_text()) {
        prop_assume!(!text.is_empty());

        let rendered = render_paragraph(&text);
        let seen: String = rendered.split_whitespace().collect();
        let meant: String = text.split_whitespace().collect();

        prop_assert!(
            seen.contains(&meant),
            "the engine displayed {seen:?}, which does not contain the source text {meant:?}"
        );
    }
}

// The same claim one tier out, with inline structure in play.
//
// The properties beside `decode` prove no source can put a marker into the
// stream. This asks the engine whether structure built from that stream stays
// structure — whether a styled run renders as text under a face, and not as an
// instruction the source talked the emitter into writing.

use adocpdf_core::document::{InlineNode, InlineStyle};

/// Renders one styled paragraph and returns what the PDF says it contains.
fn render_styled(style: InlineStyle, text: &str) -> String {
    let content = InlineText::from_nodes(vec![
        InlineNode::text("before "),
        InlineNode::styled(style, vec![InlineNode::text(text)]),
        InlineNode::text(" after"),
    ]);
    let document = Document::new().with_block(Block::Paragraph(Paragraph::new(content)));
    let plan = plan_document(&document, &ThemeSet::default()).expect("a paragraph plans");
    let bytes = TypstRenderer::new()
        .render(&plan, "property.adoc", Date::new(2026, 8, 16).unwrap())
        .expect("a styled paragraph renders");

    text_of(&bytes)
}

proptest! {
    // Six styles times a full layout and PDF export each, so the case count is
    // lower still than the tier's other property.
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// Text inside a style is still text: it survives to the page unchanged,
    /// and the words around it survive with it.
    #[test]
    fn styled_text_survives_the_real_rendering_path(text in renderable_text()) {
        prop_assume!(!text.is_empty());

        for style in [
            InlineStyle::Strong,
            InlineStyle::Emphasis,
            InlineStyle::Monospace,
            InlineStyle::Highlight,
        ] {
            let rendered = render_styled(style, &text);
            let seen: String = rendered.split_whitespace().collect();
            let meant: String = text.split_whitespace().collect();

            prop_assert!(
                seen.contains(&meant),
                "{style:?} displayed {seen:?}, which does not contain {meant:?}"
            );
            prop_assert!(
                seen.contains("before") && seen.contains("after"),
                "{style:?} lost the text around it: {seen:?}"
            );
        }
    }
}
