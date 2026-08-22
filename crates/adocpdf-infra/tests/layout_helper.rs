//! The render helper's own tests. See `tests/layout/mod.rs`.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod layout;

#[test]
fn a_rendered_document_reports_its_text_where_the_page_puts_it() {
    let pages = layout::render("Hello.\n");
    let page = pages.first().expect("one page");

    let run = page.runs.first().expect("the paragraph reached the page");

    assert!(run.text.contains("Hello"), "got: {:?}", run.text);
    assert_eq!(run.family, "DejaVu Sans", "the default body family");
    assert!(
        (run.size - 11.0).abs() < 0.01,
        "the default body size, got {}",
        run.size
    );
    assert!(
        run.x > 0.0 && run.x < page.width && run.y > 0.0 && run.y < page.height,
        "the run must sit inside the page, got ({}, {}) on {}x{}",
        run.x,
        run.y,
        page.width,
        page.height
    );
}

#[test]
fn a_two_line_paragraph_is_reported_as_two_lines_in_order() {
    // Long enough that the measure must break it, and worded so the two halves
    // cannot be confused with each other.
    let pages = layout::render(
        "Alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha \
         alpha alpha omega omega omega omega omega omega omega omega omega.\n",
    );
    let page = pages.first().expect("one page");

    let lines = page.lines();

    assert_eq!(lines.len(), 2, "got: {:?}", page.line_texts());
    assert!(
        lines[0].text.contains("Alpha"),
        "the first line comes first, got: {:?}",
        lines[0].text
    );
    assert!(
        lines[1].text.contains("omega"),
        "the second line comes second, got: {:?}",
        lines[1].text
    );
    assert!(
        lines[1].y > lines[0].y,
        "lines are ordered down the page, got {} then {}",
        lines[0].y,
        lines[1].y
    );
}

#[test]
fn runs_on_one_line_share_a_baseline_and_runs_on_two_do_not() {
    // `word` is set in a different family and arrives as its own run, so this
    // also checks that a size change on one line does not split it in two.
    let pages = layout::render("A mono `word` here.\n\nA second paragraph.\n");
    let page = pages.first().expect("one page");

    let mono = layout::run_containing(page, "word");
    let same = layout::run_containing(page, "here");
    let other = layout::run_containing(page, "second");

    assert!(
        layout::share_baseline(mono, same),
        "one line, got {} and {}",
        mono.y,
        same.y
    );
    assert!(
        !layout::share_baseline(mono, other),
        "different paragraphs, got {} and {}",
        mono.y,
        other.y
    );
}
