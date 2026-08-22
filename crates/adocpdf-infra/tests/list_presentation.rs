//! What a list's attribute list does to the page.
//!
//! A horizontal list and a run-in one both put a term before its description;
//! what tells them apart is whether the descriptions of two differently-sized
//! terms begin in the same column. That is a question about the page, and the
//! tests here ask it that way.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod layout;

use adocpdf_core::document::{Block, List};
use adocpdf_core::presentation::{ListForm, ListMarker};
use adocpdf_domain::ports::{Date, DocumentParser, ParseOutcome};
use adocpdf_infra::parser::AsciidocParser;

fn parse(source: &str) -> ParseOutcome {
    AsciidocParser
        .parse(source, "lists.adoc", Date::new(2026, 8, 22).unwrap())
        .expect("the fixture parses")
}

/// The first list of a parsed document.
fn first_list(source: &str) -> List {
    match parse(source).document.body().first() {
        Some(Block::List(list)) => list.clone(),
        other => panic!("expected a list, got {other:?}"),
    }
}

fn skipped(source: &str) -> Vec<String> {
    parse(source)
        .skipped
        .into_iter()
        .map(|skip| skip.construct)
        .collect()
}

#[test]
fn each_marker_style_is_read_as_the_shape_it_names() {
    for (style, expected) in [
        ("disc", ListMarker::Disc),
        ("circle", ListMarker::Circle),
        ("square", ListMarker::Square),
    ] {
        let list = first_list(&format!("[{style}]\n* one\n* two"));

        assert_eq!(list.presentation().marker(), Some(expected), "for {style}");
    }
}

#[test]
fn a_declared_start_is_read_from_the_attribute_list() {
    let list = first_list("[start=4]\n. one\n. two");

    assert_eq!(list.presentation().start().get(), 4);
}

#[test]
fn a_list_that_declares_no_start_counts_from_one() {
    assert_eq!(first_list(". one\n. two").presentation().start().get(), 1);
}

#[test]
fn each_description_list_form_is_read_as_the_form_it_names() {
    for (style, expected) in [
        ("horizontal", ListForm::Horizontal),
        ("qanda", ListForm::QuestionsAndAnswers),
    ] {
        let list = first_list(&format!("[{style}]\nTerm:: Description"));

        assert_eq!(list.presentation().form(), expected, "for {style}");
    }
}

#[test]
fn a_list_holding_a_checkbox_is_read_as_a_checklist() {
    let list = first_list("* [x] done\n* [ ] todo\n* plain");

    assert_eq!(list.presentation().form(), ListForm::Checklist);
    assert_eq!(list.items()[0].checkbox(), Some(true));
    assert_eq!(list.items()[1].checkbox(), Some(false));
    assert_eq!(
        list.items()[2].checkbox(),
        None,
        "an item with no box of its own has no state to show"
    );
}

#[test]
fn the_checkbox_syntax_does_not_survive_into_an_item_s_text() {
    let list = first_list("* [x] done\n* [ ] todo");

    for item in list.items() {
        let text: String = item
            .body()
            .iter()
            .map(|block| match block {
                Block::Paragraph(paragraph) => paragraph.text().plain_text(),
                other => panic!("expected a paragraph in the item, got {other:?}"),
            })
            .collect();

        assert!(
            !text.contains('['),
            "the marker syntax is not text, got {text:?}"
        );
    }
}

#[test]
fn an_ordered_list_counts_from_the_start_it_declares() {
    let pages = layout::render("[start=4]\n. one\n. two\n. three");
    let page = &pages[0];

    let numbers: Vec<String> = page
        .lines()
        .into_iter()
        .map(|line| line.text.chars().take_while(char::is_ascii_digit).collect())
        .collect();

    assert_eq!(numbers, ["4", "5", "6"], "got {:?}", page.line_texts());
}

#[test]
fn an_unordered_list_shows_the_marker_shape_it_declares() {
    for (style, glyph) in [("circle", '\u{25e6}'), ("square", '\u{25aa}')] {
        let pages = layout::render(&format!("[{style}]\n* one\n* two"));

        assert!(
            pages[0].text().contains(glyph),
            "a list declaring {style} must show {glyph:?}, got {:?}",
            pages[0].text()
        );
    }
}

#[test]
fn a_list_declaring_no_marker_keeps_the_default_one() {
    let pages = layout::render("* one\n* two");

    assert!(
        pages[0].text().contains('\u{2022}'),
        "got {:?}",
        pages[0].text()
    );
}

#[test]
fn a_horizontal_list_sets_each_description_beside_its_term() {
    let pages =
        layout::render("[horizontal]\nShort:: First answer\nA much longer term:: Second answer");
    let page = &pages[0];

    let first_term = layout::run_containing(page, "Short");
    let first_description = layout::run_containing(page, "First answer");
    let second_term = layout::run_containing(page, "A much longer term");
    let second_description = layout::run_containing(page, "Second answer");

    assert!(
        layout::share_baseline(first_term, first_description),
        "a term and its description share a line, got {} and {}",
        first_term.y,
        first_description.y
    );
    assert!(
        (first_term.x - second_term.x).abs() < 0.01,
        "the terms align with one another, got {} and {}",
        first_term.x,
        second_term.x
    );
    assert!(
        (first_description.x - second_description.x).abs() < 0.01,
        "and so do the descriptions, which is what a run-in list does not do: \
         got {} and {}",
        first_description.x,
        second_description.x
    );
}

#[test]
fn a_run_in_description_list_does_not_align_its_descriptions() {
    // The contrast the test above depends on: without `[horizontal]`, a longer
    // term pushes its description further along the line.
    let pages = layout::render("Short:: First answer\nA much longer term:: Second answer");
    let page = &pages[0];

    let first = layout::run_containing(page, "First answer");
    let second = layout::run_containing(page, "Second answer");

    assert!(
        (first.x - second.x).abs() > 1.0,
        "a run-in description follows its term, got {} and {}",
        first.x,
        second.x
    );
}

#[test]
fn a_questions_and_answers_list_numbers_its_questions() {
    let pages = layout::render("[qanda]\nWhat is it?:: A thing.\nWhy?:: Because.");
    let page = &pages[0];

    let question = layout::run_containing(page, "What is it?");
    let answer = layout::run_containing(page, "A thing.");

    assert!(
        page.text().contains("1."),
        "each question is numbered, got {:?}",
        page.text()
    );
    assert!(
        answer.y > question.y,
        "the answer sits beneath its question, got {} and {}",
        answer.y,
        question.y
    );
}

#[test]
fn a_checklist_shows_which_items_are_done() {
    let pages = layout::render("* [x] done\n* [ ] todo");
    let page = &pages[0];

    let text = page.text();

    assert!(
        text.contains('\u{2611}'),
        "a checked item shows a checked box, got {text:?}"
    );
    assert!(
        text.contains('\u{2610}'),
        "an unchecked item shows an empty one, got {text:?}"
    );
    assert!(
        !text.contains('[') && !text.contains(']'),
        "no bracket from the syntax reaches the page, got {text:?}"
    );
}

#[test]
fn a_checklist_item_s_text_sits_beside_its_box() {
    let pages = layout::render("* [x] done\n* [ ] todo");
    let page = &pages[0];

    let box_run = layout::run_containing(page, "\u{2611}");
    let text = layout::run_containing(page, "done");

    assert!(
        layout::share_baseline(box_run, text),
        "the box and its text are one thing, got {} and {}",
        box_run.y,
        text.y
    );
    assert!(box_run.x < text.x, "the box comes first");
}

#[test]
fn a_list_attribute_this_renderer_cannot_honour_costs_no_content() {
    let source = "[loweralpha]\n. one\n. two";
    let pages = layout::render(source);

    assert!(pages[0].text().contains("one"));
    assert!(pages[0].text().contains("two"));
    assert_eq!(skipped(source), [r#"list style "loweralpha""#]);
}

#[test]
fn a_start_that_is_not_a_number_is_reported_and_the_list_still_counts() {
    let source = "[start=four]\n. one\n. two";
    let pages = layout::render(source);

    assert_eq!(skipped(source).len(), 1, "got {:?}", skipped(source));
    assert!(
        skipped(source)[0].contains("four"),
        "the report names what was declared, got {:?}",
        skipped(source)
    );
    assert!(
        pages[0].text().contains("1."),
        "and the list falls back to counting from one, got {:?}",
        pages[0].text()
    );
}

#[test]
fn an_honoured_list_attribute_is_not_reported_as_skipped() {
    for source in [
        "[circle]\n* one",
        "[start=4]\n. one",
        "[horizontal]\nTerm:: Description",
        "[qanda]\nQuestion?:: Answer",
        "* [x] done\n* [ ] todo",
    ] {
        assert!(
            skipped(source).is_empty(),
            "{source:?} reported {:?}",
            skipped(source)
        );
    }
}
