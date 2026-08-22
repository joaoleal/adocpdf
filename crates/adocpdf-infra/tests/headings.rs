//! Telling one heading level from another.
//!
//! The engine sets every level below the second at body size, so a renderer
//! that names no size produces a document whose third, fourth and fifth levels
//! are indistinguishable — the structure is in the source and not on the page.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod layout;

use adocpdf_core::document::{Block, HeadingLevel};
use adocpdf_domain::ports::{Date, DocumentParser};
use adocpdf_infra::parser::AsciidocParser;

/// A section containing a discrete heading, then a paragraph, then a sibling.
const WITH_A_DISCRETE_HEADING: &str = "== One\n\n\
                                       [discrete]\n=== Aside\n\n\
                                       Body under the aside.\n\n\
                                       == Two\n\nMore body.\n";

fn body_of(source: &str) -> Vec<Block> {
    AsciidocParser
        .parse(source, "headings.adoc", Date::new(2026, 8, 22).unwrap())
        .expect("the fixture parses")
        .document
        .body()
        .to_vec()
}

/// A document with a heading at every level the renderer honours.
const EVERY_LEVEL: &str = "= Title\n\nBody text.\n\n\
                           == Level one\n\nBody.\n\n\
                           === Level two\n\nBody.\n\n\
                           ==== Level three\n\nBody.\n\n\
                           ===== Level four\n\nBody.\n\n\
                           ====== Level five\n\nBody.\n";

/// The size each heading was set at, in the order they appear.
fn heading_sizes() -> Vec<f64> {
    let pages = layout::render(EVERY_LEVEL);

    pages[0]
        .runs
        .iter()
        .filter(|run| run.text.contains("Level") || run.text.contains("Title"))
        .map(|run| run.size)
        .collect()
}

#[test]
fn no_two_heading_levels_are_set_identically() {
    let sizes = heading_sizes();

    assert!(sizes.len() >= 5, "expected every level, got: {sizes:?}");
    for (index, size) in sizes.iter().enumerate().skip(1) {
        assert!(
            *size < sizes[index - 1],
            "each level must be smaller than the one above it, got: {sizes:?}"
        );
    }
}

#[test]
fn every_heading_level_is_larger_than_body_text() {
    let pages = layout::render(EVERY_LEVEL);
    let body = layout::run_containing(&pages[0], "Body").size;

    for size in heading_sizes() {
        assert!(
            size > body,
            "a heading set at body size is not a heading: {size} against {body}"
        );
    }
}

#[test]
fn a_discrete_heading_is_a_heading_and_not_a_section() {
    let body = body_of(WITH_A_DISCRETE_HEADING);

    let Block::Section(first) = &body[0] else {
        panic!("expected a section, got {:?}", body[0]);
    };
    let Block::Heading { text, level } = &first.body()[0] else {
        panic!("expected a heading block, got {:?}", first.body()[0]);
    };

    assert_eq!(text.plain_text(), "Aside");
    assert_eq!(*level, HeadingLevel::new(2).unwrap());
}

#[test]
fn the_blocks_after_a_discrete_heading_belong_to_the_enclosing_section() {
    let body = body_of(WITH_A_DISCRETE_HEADING);

    let Block::Section(first) = &body[0] else {
        panic!("expected a section, got {:?}", body[0]);
    };

    let Block::Paragraph(paragraph) = &first.body()[1] else {
        panic!(
            "the paragraph must be a sibling of the discrete heading, got {:?}",
            first.body()
        );
    };
    assert_eq!(paragraph.text().plain_text(), "Body under the aside.");
    assert_eq!(
        first.body().len(),
        2,
        "and the discrete heading must contain nothing of its own"
    );
}

#[test]
fn a_section_after_a_discrete_heading_keeps_the_level_it_would_have_had() {
    let body = body_of(WITH_A_DISCRETE_HEADING);

    let (Block::Section(first), Block::Section(second)) = (&body[0], &body[1]) else {
        panic!("expected two sections, got {body:?}");
    };

    assert_eq!(first.level(), second.level());
    assert_eq!(second.heading().plain_text(), "Two");
}

#[test]
fn a_discrete_heading_is_set_as_a_heading() {
    let pages = layout::render(WITH_A_DISCRETE_HEADING);
    let page = &pages[0];

    let discrete = layout::run_containing(page, "Aside");
    let body = layout::run_containing(page, "Body under the aside.");

    assert!(
        discrete.size > body.size,
        "a discrete heading is still a heading, got {} against {}",
        discrete.size,
        body.size
    );
}

#[test]
fn a_discrete_heading_does_not_disturb_the_headings_around_it() {
    let pages = layout::render(WITH_A_DISCRETE_HEADING);
    let page = &pages[0];

    let before = layout::run_containing(page, "One");
    let after = layout::run_containing(page, "Two");
    let discrete = layout::run_containing(page, "Aside");

    assert!(
        (before.size - after.size).abs() < f64::EPSILON,
        "the sections on either side are set at one level's size, got {} and {}",
        before.size,
        after.size
    );
    assert!(
        discrete.size < before.size,
        "and the discrete heading is set at its own declared depth, got {} against {}",
        discrete.size,
        before.size
    );
}
