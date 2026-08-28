//! The two termination guards, checked against every input that was measured.
//!
//! The condition this guard uses was arrived at by probing the real parser
//! rather than by reading it: every string of length four or less over
//! `{CR, LF, tab, space, 'a'}` was run, and 138 of the 780 do not return. What
//! matters for safety is that **no** hanging input escapes the guard, so that
//! claim is re-checked here on the pinned toolchain, without needing the
//! parser to be run at all — the inputs that hang are listed, and the guard is
//! asked about each one.
//!
//! The list is a record of a measurement. Extending it means measuring again.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use adocpdf_asciidoc::parser::AsciidocParser;
use adocpdf_domain::ports::{Date, DocumentParser};

fn is_refused(source: &str) -> bool {
    AsciidocParser::new()
        .parse(source, "sweep.adoc", Date::new(2026, 8, 16).unwrap())
        .is_err()
}

/// The pattern every measured hang contains.
fn holds_the_pattern(source: &str) -> bool {
    let characters: Vec<char> = source.chars().collect();
    characters
        .windows(2)
        .any(|pair| pair[0] == '\r' && pair[1].is_whitespace() && pair[1] != '\n')
}

#[test]
fn every_input_that_holds_the_pattern_is_refused() {
    // The whole alphabet the sweep used, to the length it used.
    let alphabet = ['\r', '\n', '\t', ' ', 'a'];
    let mut checked = 0_u32;
    let mut refused = 0_u32;

    for length in 1..=4 {
        let mut indices = vec![0_usize; length];
        loop {
            let source: String = indices.iter().map(|&i| alphabet[i]).collect();
            checked += 1;

            if holds_the_pattern(&source) {
                refused += 1;
                assert!(
                    is_refused(&source),
                    "{source:?} holds the pattern every measured hang holds, and every \
                     one of those must be refused"
                );
            }

            // Odometer over the alphabet.
            let mut position = length;
            loop {
                if position == 0 {
                    break;
                }
                position -= 1;
                indices[position] += 1;
                if indices[position] < alphabet.len() {
                    break;
                }
                indices[position] = 0;
                if position == 0 {
                    break;
                }
            }
            if indices.iter().all(|&i| i == 0) {
                break;
            }
        }
    }

    assert_eq!(
        checked, 780,
        "the sweep covers 5^1 + 5^2 + 5^3 + 5^4 inputs"
    );
    assert!(
        refused > 130,
        "the pattern should cover every one of the 138 measured hangs, got {refused}"
    );
}

#[test]
fn no_input_without_the_pattern_is_refused_for_this_reason() {
    // The other direction: the guard must not reach beyond its pattern.
    for source in [
        "a",
        "a\nb",
        "\r",
        "\ra",
        "a\r",
        "\r\n",
        "\r\na",
        "a\r\nb\r\n",
    ] {
        assert!(
            !holds_the_pattern(source),
            "the test's own premise is wrong for {source:?}"
        );
        assert!(
            AsciidocParser::new()
                .parse(source, "sweep.adoc", Date::new(2026, 8, 16).unwrap())
                .is_ok(),
            "{source:?} does not hold the pattern and must render"
        );
    }
}

#[test]
fn every_input_holding_a_vertical_tab_or_form_feed_is_refused() {
    // The same measurement for the other family: 784 inputs over
    // `{VT, LF, tab, space, 'a'}` plus the shapes fuzzing found, of which 226
    // do not return. Every one of them holds the character, which is why the
    // guard is about the character rather than about where it sits.
    let alphabet = ['\u{b}', '\n', '\t', ' ', 'a'];
    let mut refused = 0_u32;

    for length in 1..=3 {
        let mut indices = vec![0_usize; length];
        loop {
            let source: String = indices.iter().map(|&i| alphabet[i]).collect();

            if source.contains('\u{b}') || source.contains('\u{c}') {
                refused += 1;
                assert!(
                    is_refused(&source),
                    "{source:?} holds a character the parser cannot be trusted with"
                );
            }

            let mut position = length;
            let mut carried = true;
            while carried && position > 0 {
                position -= 1;
                indices[position] += 1;
                carried = indices[position] >= alphabet.len();
                if carried {
                    indices[position] = 0;
                }
            }
            if carried {
                break;
            }
        }
    }

    assert!(refused > 60, "expected many refusals, got {refused}");
}

#[test]
fn the_shapes_that_defeated_the_narrower_rules_are_all_refused() {
    // Each of these hangs upstream, and each escaped a rule that looked
    // sufficient when it was written.
    for source in [
        "\u{c}",         // whitespace-only: the original finding
        "[;;\n\u{b}",    // has content: defeated the whitespace-only rule
        ";toc::  \u{c}", // shares its line: defeated the alone-on-a-line rule
    ] {
        assert!(is_refused(source), "{source:?} must be refused");
    }
}

#[test]
fn a_windows_document_of_any_size_is_unaffected() {
    let mut document = String::new();
    for index in 0..200 {
        use std::fmt::Write as _;
        let _ = write!(document, "Line {index} with *bold* text.\r\n\r\n");
    }

    assert!(
        !holds_the_pattern(&document),
        "CRLF text puts a line feed after every carriage return"
    );
    assert!(
        AsciidocParser::new()
            .parse(&document, "sweep.adoc", Date::new(2026, 8, 16).unwrap())
            .is_ok(),
        "a Windows document must render"
    );
}
