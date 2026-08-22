//! Where a paragraph's lines end.
//!
//! AsciiDoc fills paragraphs: an ordinary newline inside one is a soft wrap the
//! author's editor put there, not a break the reader should see. These tests
//! read the laid-out page, because that is the only place the difference shows
//! — the emitted markup looks identical either way, which is how the defect
//! these tests were written for survived a suite of markup assertions.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod layout;

/// The same prose, wrapped in the source and not.
///
/// Wrapped at a deliberately narrow width, well short of the measure, so that
/// honouring the source's newlines and honouring the measure cannot produce the
/// same answer by accident.
const WRAPPED: &str = "Alpha alpha alpha alpha alpha\n\
                       alpha alpha alpha alpha alpha\n\
                       alpha alpha alpha alpha omega.\n";

const UNWRAPPED: &str = "Alpha alpha alpha alpha alpha alpha alpha alpha alpha \
                         alpha alpha alpha alpha alpha omega.\n";

#[test]
fn source_wrapping_does_not_reach_the_page() {
    let wrapped = layout::render(WRAPPED);
    let unwrapped = layout::render(UNWRAPPED);

    assert_eq!(
        wrapped[0].line_texts(),
        unwrapped[0].line_texts(),
        "the same paragraph must render the same however its source is wrapped"
    );
}

#[test]
fn a_wrapped_paragraph_fills_the_measure() {
    // Stated separately from the comparison above so that a failure says which
    // of the two is wrong. Three source lines that fit on two rendered ones
    // prove the text was refilled rather than passed through.
    let pages = layout::render(WRAPPED);
    let lines = pages[0].line_texts();

    assert!(
        lines.len() < 3,
        "three short source lines must be refilled to fewer, got: {lines:?}"
    );
}

#[test]
fn a_wider_measure_produces_longer_lines() {
    // The same paragraph, once on A4 and once on the landscape `wide` theme.
    // Before paragraphs were filled this test could not have failed: the text
    // arrived pre-broken at the portrait measure and the wider page simply left
    // the right half of itself empty.
    let narrow = layout::render_with(WRAPPED, &layout::built_in_themes());
    let wide = layout::render_with(
        &format!("[theme=wide]\n== Wide\n\n{WRAPPED}"),
        &layout::built_in_themes(),
    );

    // Counted over the paragraph's own lines: the wide fixture needs a section
    // to hang the theme on, and its heading is a line like any other.
    let paragraph_lines = |page: &layout::Page| {
        page.line_texts()
            .into_iter()
            .filter(|line| line != "Wide")
            .count()
    };

    assert!(
        paragraph_lines(&wide[0]) < paragraph_lines(&narrow[0]),
        "the wider page must use fewer lines: {:?} against {:?}",
        wide[0].line_texts(),
        narrow[0].line_texts()
    );
}

#[test]
fn a_hard_break_breaks_and_a_source_wrap_does_not() {
    // Both kinds of line ending in one paragraph. The first must break the
    // line; the second must not.
    let pages = layout::render("Alpha alpha alpha +\nbeta beta beta\nbeta beta beta beta.\n");
    let lines = pages[0].line_texts();

    assert_eq!(
        lines.len(),
        2,
        "one break, and one wrap that is not a break, got: {lines:?}"
    );
    assert_eq!(
        lines[0], "Alpha alpha alpha",
        "the hard break ends the line"
    );
    assert_eq!(
        lines[1], "beta beta beta beta beta beta beta.",
        "the source wrap is filled away"
    );
}
