//! What a role does to the page.
//!
//! An underlined run and an ordinary one are the same text run in the frame:
//! the rule is a separate shape drawn beside it. So asking a run whether it was
//! underlined cannot answer, and a test that read `underline("x")` out of the
//! markup would know only that the emitter said so. These tests look for the
//! rule.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod layout;

use adocpdf_asciidoc::parser::AsciidocParser;
use adocpdf_domain::ports::{Date, DocumentParser};

/// What rendering `source` reported as skipped.
fn skipped(source: &str) -> Vec<String> {
    AsciidocParser
        .parse(source, "roles.adoc", Date::new(2026, 8, 22).unwrap())
        .expect("the fixture parses")
        .skipped
        .into_iter()
        .map(|skip| skip.construct)
        .collect()
}

#[test]
fn an_underlined_span_carries_a_rule_beneath_it() {
    let pages = layout::render("[.underline]#ruled# and plain.");
    let page = &pages[0];
    let run = layout::run_containing(page, "ruled");

    let rules = page.rules_over(run);

    assert_eq!(
        rules.len(),
        1,
        "expected one rule under the span, got {rules:?}"
    );
    assert!(
        rules[0].y > run.y,
        "an underline sits below the baseline, got {} against {}",
        rules[0].y,
        run.y
    );
}

#[test]
fn a_struck_span_carries_a_rule_through_it() {
    let pages = layout::render("[.line-through]#struck# and plain.");
    let page = &pages[0];
    let run = layout::run_containing(page, "struck");

    let rules = page.rules_over(run);

    assert_eq!(rules.len(), 1, "expected one rule, got {rules:?}");
    assert!(
        rules[0].y < run.y && run.y - rules[0].y < run.size,
        "a strikethrough crosses the letters rather than sitting under or above \
         them, got {} against a baseline of {} at {}pt",
        rules[0].y,
        run.y,
        run.size
    );
}

#[test]
fn a_plain_span_carries_no_rule_at_all() {
    let pages = layout::render("Nothing here is decorated.");

    assert!(
        pages[0].rules.is_empty(),
        "undecorated text drew {:?}",
        pages[0].rules
    );
}

#[test]
fn the_relative_size_roles_set_a_span_apart_from_body_text() {
    let pages = layout::render("body [.big]#large# body [.small]#tiny# body");
    let page = &pages[0];

    let body = layout::run_containing(page, "body").size;
    let larger = layout::run_containing(page, "large").size;
    let smaller = layout::run_containing(page, "tiny").size;

    assert!(
        larger > body,
        "the big role must set larger than body text, got {larger} against {body}"
    );
    assert!(
        smaller < body,
        "the small role must set smaller than body text, got {smaller} against {body}"
    );
}

#[test]
fn a_role_composes_with_a_style() {
    let pages = layout::render("[.underline]*both* and plain.");
    let page = &pages[0];
    let run = layout::run_containing(page, "both");
    let plain = layout::run_containing(page, "plain");

    assert_eq!(page.rules_over(run).len(), 1, "the role must still rule it");
    assert!(
        run.weight > plain.weight,
        "the span must still be bold, got weight {} against {}",
        run.weight,
        plain.weight
    );
}

#[test]
fn a_role_this_renderer_cannot_honour_keeps_its_text_and_is_named() {
    let source = "Before [.warning]#the words# after.";
    let pages = layout::render(source);
    let page = &pages[0];

    assert!(
        page.text().contains("the words"),
        "the text an unhonoured role enclosed must still reach the page, got {:?}",
        page.text()
    );
    assert_eq!(skipped(source), [r#"role "warning""#]);
}

#[test]
fn an_unhonoured_role_leaves_the_text_set_as_body_text() {
    let pages = layout::render("Before [.warning]#the words# after.");
    let page = &pages[0];

    let enclosed = layout::run_containing(page, "the words");
    let around = layout::run_containing(page, "Before");

    assert!(
        (enclosed.size - around.size).abs() < f64::EPSILON,
        "an unhonoured role must not invent a size, got {} against {}",
        enclosed.size,
        around.size
    );
    assert_eq!(enclosed.family, around.family);
    assert!(page.rules_over(enclosed).is_empty(), "nor a decoration");
}

#[test]
fn an_honoured_role_is_not_reported_as_skipped() {
    for source in [
        "[.underline]#x#",
        "[.line-through]#x#",
        "[.big]#x#",
        "[.small]#x#",
    ] {
        assert!(
            skipped(source).is_empty(),
            "{source} reported {:?}",
            skipped(source)
        );
    }
}
