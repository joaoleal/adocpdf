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
