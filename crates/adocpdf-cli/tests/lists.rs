//! How a list sits on the page.
//!
//! A marker and the text it marks are one thing to a reader, and the questions
//! here are all about that relationship: are they on the same line, does a
//! wrapped item align under its text, is a nested level further in. None of
//! them can be asked of emitted markup, which is why the marker spent a release
//! sitting on a line of its own without a single test noticing.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod layout;

#[test]
fn an_unordered_marker_sits_beside_the_text_it_marks() {
    let pages = layout::render("* alpha\n* beta\n");
    let page = &pages[0];

    let marker = layout::run_containing(page, "•");
    let text = layout::run_containing(page, "alpha");

    assert!(
        layout::share_baseline(marker, text),
        "the marker and its text must be on one line, got {} and {}",
        marker.y,
        text.y
    );
    assert!(
        marker.x < text.x,
        "the marker comes first, got {} then {}",
        marker.x,
        text.x
    );
}

#[test]
fn no_line_holds_a_marker_and_nothing_else() {
    let pages = layout::render("* alpha\n* beta\n");

    for line in pages[0].lines() {
        assert!(
            line.text.trim() != "•",
            "a marker alone on a line, in: {:?}",
            pages[0].line_texts()
        );
    }
}

#[test]
fn an_ordered_marker_sits_beside_the_text_it_marks() {
    let pages = layout::render(". alpha\n. beta\n");
    let page = &pages[0];

    let marker = layout::run_containing(page, "1.");
    let text = layout::run_containing(page, "alpha");

    assert!(
        layout::share_baseline(marker, text),
        "the number and its text must be on one line, got {} and {}",
        marker.y,
        text.y
    );
}

#[test]
fn a_description_term_and_its_description_both_reach_the_page() {
    let pages = layout::render("apple:: a fruit\npear:: another fruit\n");
    let text = pages[0].text();

    assert!(text.contains("apple"), "got: {text:?}");
    assert!(text.contains("a fruit"), "got: {text:?}");
    assert!(text.contains("pear"), "got: {text:?}");
}

#[test]
fn an_ordered_lists_numbers_are_the_ones_the_renderer_determined() {
    let pages = layout::render(". alpha\n. beta\n. gamma\n");
    let text = pages[0].text();

    for number in ["1.", "2.", "3."] {
        assert!(
            text.contains(number),
            "expected {number} on the page, got: {text:?}"
        );
    }
}

#[test]
fn a_nested_list_is_indented_relative_to_its_parent() {
    let pages = layout::render("* outer\n** inner\n* outer again\n");
    let page = &pages[0];

    let outer = layout::run_containing(page, "outer");
    let inner = layout::run_containing(page, "inner");

    assert!(
        inner.x > outer.x,
        "the nested level must be further in, got {} against {}",
        inner.x,
        outer.x
    );
}

#[test]
fn a_wrapped_item_aligns_under_its_text_not_under_its_marker() {
    // Hanging indent. The hand-built blocks this replaced had none: a second
    // line began under the bullet, so a long item read as a paragraph with a
    // bullet loose above it. No sample short enough to fit on one line could
    // show it, which is why it went unnoticed.
    let pages = layout::render(
        "* Alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha \
         alpha alpha alpha alpha alpha alpha omega omega omega omega omega.\n",
    );
    let lines = pages[0].lines();

    assert_eq!(lines.len(), 2, "got: {:?}", pages[0].line_texts());
    assert!(
        lines[1].left > lines[0].left,
        "the second line must hang under the text, not under the marker: \
         {} against {}",
        lines[1].left,
        lines[0].left
    );
}

#[test]
fn no_marker_is_written_as_text_for_the_engine_to_read_back() {
    // The markup half of the claim above. An emitted `1. ` would be read by the
    // engine as its own enumeration syntax rather than as the text it is, and
    // the number on the page would then be one this renderer never chose.
    let markup = layout::markup(". alpha\n. beta\n\n* gamma\n");

    assert!(
        markup.contains("enum.item(1, "),
        "the renderer's own position must be passed, got:\n{markup}"
    );
    assert!(
        markup.contains("enum.item(2, "),
        "and for every item, got:\n{markup}"
    );
    assert!(
        markup.contains("list.item("),
        "an unordered item is a list item, got:\n{markup}"
    );
    assert!(
        !markup.contains("[• "),
        "no bullet is written as text, got:\n{markup}"
    );
}

#[test]
fn an_items_text_reaches_the_page_exactly_as_written() {
    // Found by looking at a rendered page, not by a test. An item holding a
    // nested list takes the content-block path, and a content block is markup
    // mode — where the emitter's own code expression was read as literal text,
    // quotation marks of the string literal included, which the engine then
    // curled into smart quotes. Every item below read `“like this”`.
    let pages = layout::render("* outer item\n** inner item\n* plain item\n");

    for expected in ["outer item", "inner item", "plain item"] {
        let text = pages[0].text();
        assert!(text.contains(expected), "got: {text:?}");
    }
    assert!(
        !pages[0].text().contains('\u{201c}') && !pages[0].text().contains('"'),
        "no quotation mark may be added to an item's text, got: {:?}",
        pages[0].text()
    );
}
