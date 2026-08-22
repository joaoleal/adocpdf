//! What a paragraph's attribute list does to the page.
//!
//! Alignment is a geometric claim and nothing else: a centred paragraph is one
//! whose lines sit in from both margins by the same amount. The markup that
//! produced it says `align(center, …)`, which is a claim about the emitter, not
//! about where the words landed.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod layout;

use adocpdf_core::document::{Block, Paragraph};
use adocpdf_core::presentation::Alignment;
use adocpdf_domain::ports::{Date, DocumentParser};
use adocpdf_infra::parser::AsciidocParser;

use layout::Line;

/// Enough words that a paragraph fills several lines.
const BODY: &str = "The quick brown fox jumps over the lazy dog, and then it \
                    keeps going for long enough that the paragraph has to break \
                    across more than one line of the measure.";

/// A second paragraph of the same length, told apart by its first word.
const OTHER: &str = "Meanwhile the slow grey badger ambles past the sleeping \
                     hound, taking its time, and filling just as many lines as \
                     the paragraph above it does.";

/// The first paragraph of a parsed document.
fn first_paragraph(source: &str) -> Paragraph {
    let outcome = AsciidocParser
        .parse(source, "paragraphs.adoc", Date::new(2026, 8, 22).unwrap())
        .expect("the fixture parses");

    match outcome.document.body().first() {
        Some(Block::Paragraph(paragraph)) => paragraph.clone(),
        other => panic!("expected a paragraph, got {other:?}"),
    }
}

/// The lines from the one beginning `opening` up to the one beginning `until`.
///
/// Paragraphs cannot be told apart by the gaps between their lines: the space
/// between two paragraphs here is *smaller* than the leading inside one,
/// because the leading is generous and the paragraph spacing is not. So the
/// fixtures name where each paragraph starts, and this reads between the names.
fn lines_from(page: &layout::Page, opening: &str, until: Option<&str>) -> Vec<Line> {
    let all = page.lines();
    let start = all
        .iter()
        .position(|line| line.text.starts_with(opening))
        .unwrap_or_else(|| panic!("no line begins {opening:?}: {:?}", page.line_texts()));

    all[start..]
        .iter()
        .take_while(|line| until.is_none_or(|stop| !line.text.starts_with(stop)))
        .cloned()
        .collect()
}

/// Where the measure ends: the right margin, mirrored from the left one.
///
/// A fixture whose aligned line ends in punctuation cannot use this: the engine
/// hangs a final full stop into the margin, so the line's advance genuinely
/// runs past the measure by the width of the stop. The fixtures here end their
/// aligned lines on a letter.
fn measure_right(page: &layout::Page) -> f64 {
    let left = page
        .lines()
        .iter()
        .map(|line| line.left)
        .fold(f64::INFINITY, f64::min);

    page.width - left
}

#[test]
fn each_alignment_role_is_read_as_the_alignment_it_names() {
    for (role, expected) in [
        ("text-left", Alignment::Left),
        ("text-center", Alignment::Center),
        ("text-right", Alignment::Right),
        ("text-justify", Alignment::Justify),
    ] {
        let paragraph = first_paragraph(&format!("[.{role}]\nWords."));

        assert_eq!(
            paragraph.presentation().alignment(),
            Some(expected),
            "for {role}"
        );
        assert!(!paragraph.presentation().is_lead(), "for {role}");
    }
}

#[test]
fn the_lead_role_is_read_as_a_lead_paragraph() {
    let paragraph = first_paragraph("[.lead]\nWords.");

    assert!(paragraph.presentation().is_lead());
    assert_eq!(paragraph.presentation().alignment(), None);
}

#[test]
fn a_paragraph_that_declares_nothing_is_body_text() {
    assert!(first_paragraph("Words.").presentation().is_body());
}

#[test]
fn a_centred_paragraph_is_centred_on_the_measure() {
    let pages = layout::render(&format!("{BODY}\n\n[.text-center]\nCentred words\n"));
    let page = &pages[0];

    let body = &lines_from(page, "The quick", Some("Centred"))[0];
    let centred = &lines_from(page, "Centred words", None)[0];

    let left_inset = centred.left - body.left;
    let right_inset = measure_right(page) - centred.right;

    assert!(
        left_inset > 1.0,
        "a centred line starts in from the margin, got {left_inset}"
    );
    assert!(
        (left_inset - right_inset).abs() < 1.0,
        "a centred line is inset equally at both ends, got {left_inset} and {right_inset}"
    );
}

#[test]
fn an_alignment_reaches_only_the_paragraph_that_declared_it() {
    let pages = layout::render(&format!("{BODY}\n\n[.text-center]\nCentred.\n\n{OTHER}"));
    let page = &pages[0];

    let before = &lines_from(page, "The quick", Some("Centred"))[0];
    let after = &lines_from(page, "Meanwhile", None)[0];

    assert!(
        (before.left - after.left).abs() < 0.01,
        "the paragraphs on either side must start in the same column, got {} and {}",
        before.left,
        after.left
    );
}

#[test]
fn a_right_aligned_paragraph_ends_at_the_right_margin() {
    let pages = layout::render(&format!("{BODY}\n\n[.text-right]\nShort line\n"));
    let page = &pages[0];

    let body = &lines_from(page, "The quick", Some("Short line"))[0];
    let aligned = &lines_from(page, "Short line", None)[0];

    assert!(
        (aligned.right - measure_right(page)).abs() < 1.0,
        "a right-aligned line ends at the measure, got {} against {}",
        aligned.right,
        measure_right(page)
    );
    assert!(
        aligned.left > body.left + 1.0,
        "and starts further in than a full line, got {} against {}",
        aligned.left,
        body.left
    );
}

#[test]
fn a_justified_paragraph_is_flush_under_a_theme_that_does_not_justify() {
    let pages = layout::render(&format!("{BODY}\n\n[.text-justify]\n{OTHER}\n"));
    let page = &pages[0];

    // The last line of a justified paragraph is never stretched, so it says
    // nothing about whether the paragraph was justified.
    let interior = |opening: &str, until: Option<&str>| -> Vec<f64> {
        let mut lines = lines_from(page, opening, until);
        lines.pop();
        lines.iter().map(|line| line.right).collect()
    };

    let ragged = interior("The quick", Some("Meanwhile"));
    let justified = interior("Meanwhile", None);

    assert!(
        !ragged.is_empty() && !justified.is_empty(),
        "both paragraphs must wrap for this to mean anything, got {ragged:?} and {justified:?}"
    );
    for right in &justified {
        assert!(
            (right - measure_right(page)).abs() < 0.5,
            "every interior line of a justified paragraph reaches the measure, \
             got {justified:?} against {}",
            measure_right(page)
        );
    }
    assert!(
        ragged
            .iter()
            .any(|right| (right - measure_right(page)).abs() > 1.0),
        "the rest of the document stays ragged-right, got {ragged:?} against {}",
        measure_right(page)
    );
}

#[test]
fn a_lead_paragraph_is_set_apart_from_the_body_text_around_it() {
    let pages = layout::render("[.lead]\nThe opening.\n\nOrdinary body text.");
    let page = &pages[0];

    let lead = layout::run_containing(page, "The opening.");
    let body = layout::run_containing(page, "Ordinary body text.");

    assert!(
        lead.size > body.size,
        "a lead paragraph must be distinguishable from body text, got {} against {}",
        lead.size,
        body.size
    );
}
