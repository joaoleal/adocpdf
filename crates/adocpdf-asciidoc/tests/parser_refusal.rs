//! The parse boundary's refusal, from both sides.
//!
//! `tests/hang_regressions.rs` records the inputs fuzzing found and asserts
//! they terminate. This file describes the guard that makes them terminate:
//! what it refuses, and — the part worth testing — what it deliberately does
//! not.
//!
//! The distinction is the whole design. Each guard here started as a narrow
//! rule fitted to the inputs in hand and was outflanked by the next fuzzing
//! run, so the rules that survive are the structural ones — about a character,
//! or a character followed by whitespace — and each test below says both what
//! it refuses and what it deliberately leaves alone.
//!
//! The last group is not a guard at all: it records a parser *panic*, which no
//! rule about the source text can predict, caught at the parse call instead.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use adocpdf_asciidoc::parser::AsciidocParser;
use adocpdf_core::document::Block;
use adocpdf_domain::error::DomainError;
use adocpdf_domain::ports::{Date, DocumentParser, ParseOutcome};

fn parse(source: &str) -> Result<ParseOutcome, DomainError> {
    let today = Date::new(2026, 8, 16).expect("a real date");
    AsciidocParser.parse(source, "boundary.adoc", today)
}

/// Every paragraph's text in a parsed document, joined.
fn plain_text(outcome: &ParseOutcome) -> String {
    let mut out = String::new();
    for block in outcome.document.body() {
        if let Block::Paragraph(paragraph) = block {
            out.push_str(&paragraph.text().plain_text());
            out.push('\n');
        }
    }
    out
}

#[test]
fn a_whitespace_only_document_containing_a_form_feed_is_refused() {
    let error = parse("\u{c}").expect_err("this input cannot be parsed without hanging");

    assert!(
        matches!(error, DomainError::ParseFailed { .. }),
        "expected a parse failure, got {error:?}"
    );
    assert!(
        error.to_string().contains("U+000C"),
        "the error must name the character, got: {error}"
    );
}

#[test]
fn a_whitespace_only_document_containing_a_vertical_tab_is_refused() {
    let error = parse("  \u{b}\n").expect_err("this input cannot be parsed without hanging");

    assert!(
        error.to_string().contains("U+000B"),
        "the error must name the character, got: {error}"
    );
}

#[test]
fn a_vertical_tab_or_form_feed_is_refused_wherever_it_appears() {
    // These four all render today, and `"Hello\u{c}world"` was previously
    // asserted here as an allowance that must be preserved. It is refused now,
    // deliberately: see the guard's own documentation for why three narrower
    // rules were tried and each was outflanked by the next fuzzing run.
    for source in [
        "Hello\u{c}world",
        "a\u{c}",
        "\u{c}a",
        "\u{b}text",
        // And the shapes that do hang, which the narrower rules missed in turn.
        "[;;\n\u{b}",
        "= T\n\n\u{b}",
        ";toc::  \u{c}",
        "a\n\u{c}",
    ] {
        assert!(
            parse(source).is_err(),
            "a vertical tab or form feed must be refused wherever it sits: {source:?}"
        );
    }
}

#[test]
fn the_refusal_is_bounded_to_those_two_characters() {
    // Every other control character, and ordinary text, still parses. The
    // refusal is a rule about two characters, not about control characters in
    // general.
    for source in ["a\tb", "a\u{7}b", "a\u{1b}b", "a\u{85}b", "ordinary text"] {
        assert!(
            parse(source).is_ok(),
            "this document must still parse: {source:?}"
        );
    }
}

#[test]
fn ordinary_empty_documents_are_still_accepted() {
    for source in ["", "   ", "\n", "\n\n  \t "] {
        assert!(
            parse(source).is_ok(),
            "{source:?} contains no offending character and must be accepted"
        );
    }
}

#[test]
fn the_refusal_names_the_document_it_refused() {
    let error = parse("\u{c}").expect_err("refused");

    assert!(
        error.to_string().contains("boundary.adoc"),
        "the error must name the document, got: {error}"
    );
}

#[test]
fn the_refusal_is_distinguishable_from_a_syntax_error() {
    let error = parse("\u{c}").expect_err("refused");
    let message = error.to_string();

    // An author reading this needs to know the document was declined because
    // of a defect in the parser, not because they wrote something invalid.
    assert!(
        message.contains("asciidoc-parser"),
        "the message must say what could not handle it, got: {message}"
    );
    assert!(
        message.contains("terminate"),
        "the message must say why it was declined, got: {message}"
    );
}

// The second guard: characters reserved for marking inline structure.
//
// This one is not about termination. `pass:[…]` and `+++…+++` reach the
// parser's rendered output verbatim, so without this guard a document could
// type a structural marker straight into the stream the inline decoder reads.

#[test]
fn a_document_containing_a_structural_marker_is_refused() {
    let error =
        parse("Ordinary text \u{fdd0} more text\n").expect_err("a marker character is refused");

    assert!(
        matches!(error, DomainError::ParseFailed { .. }),
        "expected a parse failure, got {error:?}"
    );
    assert!(
        error.to_string().contains("U+FDD0"),
        "the error must name the code point, got: {error}"
    );
}

#[test]
fn the_refusal_explains_why_the_character_is_not_allowed() {
    let error = parse("\u{fdef}").expect_err("a marker character is refused");
    let message = error.to_string();

    assert!(
        message.contains("noncharacter"),
        "the message must say what the character is, got: {message}"
    );
    assert!(
        message.contains("forge"),
        "the message must say what accepting it would allow, got: {message}"
    );
    assert!(
        !message.contains("does not terminate"),
        "this must not be confused with the hang refusal, got: {message}"
    );
}

#[test]
fn every_character_in_the_reserved_range_is_refused() {
    for code_point in 0xFDD0..=0xFDEF_u32 {
        let character = char::from_u32(code_point).expect("a noncharacter is still a char");
        let source = format!("text {character} text");

        assert!(
            parse(&source).is_err(),
            "U+{code_point:04X} must be refused; a gap in the range is a gap in the guard"
        );
    }
}

#[test]
fn a_marker_inside_a_passthrough_is_refused_too() {
    // The channel the guard exists for: passthrough content bypasses every
    // substitution, so a marker typed here would otherwise reach the decoder.
    let error = parse("pass:[\u{fdd1}] and +++\u{fdd2}+++\n")
        .expect_err("a marker in a passthrough is still a marker");

    assert!(matches!(error, DomainError::ParseFailed { .. }));
}

#[test]
fn ordinary_documents_are_unaffected_by_the_marker_guard() {
    // Characters that sit near the reserved range, and characters that look
    // exotic but are perfectly ordinary content.
    for source in [
        "Plain text.\n",
        "Arabic presentation forms: \u{fdcf} and \u{fdf0}\n",
        // U+E000 used to sit in this list. It is refused now: the parser
        // reserves it, and a document containing one makes it misread its own
        // output. Private use characters outside that range still parse.
        "Emoji \u{1f600}, CJK \u{4e2d}\u{6587}, private use \u{f8ff}\n",
        "= Title\n\nBody with *bold* and _italic_.\n",
    ] {
        assert!(
            parse(source).is_ok(),
            "this document must still parse: {source:?}"
        );
    }
}

#[test]
fn the_refusal_reports_where_the_character_was_found() {
    let error = parse("line one\nline two \u{fdd0}\n").expect_err("a marker character is refused");

    let DomainError::ParseFailed { location, .. } = error else {
        panic!("expected a parse failure");
    };
    assert_eq!(location.line, 2, "the marker is on the second line");
    assert_eq!(location.column, 10, "and after nine characters of it");
}

// The third guard: a line of carriage returns.
//
// A second hang of the same family as the form feed, and a worse one: it
// affects documents that have real content, so the whitespace-only shape of
// the first guard does not cover it.

#[test]
fn a_carriage_return_followed_by_whitespace_is_refused() {
    let error = parse("\n\r\r\r").expect_err("this input cannot be parsed without hanging");

    assert!(
        matches!(error, DomainError::ParseFailed { .. }),
        "expected a parse failure, got {error:?}"
    );
    assert!(
        error.to_string().contains("carriage return"),
        "the error must say what it found, got: {error}"
    );
}

#[test]
fn every_demonstrated_hang_of_this_family_is_refused() {
    // Each of these was measured against the real parser and does not return.
    for source in [
        "\r\r",
        "\r\t",
        "\r ",
        "\n\r\r",
        "\t\r\r",
        "\r \r",
        "= T\n\n\r\r",
    ] {
        assert!(
            parse(source).is_err(),
            "this input does not terminate upstream and must be refused: {source:?}"
        );
    }
}

#[test]
fn a_document_with_real_content_is_refused_too_when_it_holds_such_a_line() {
    // The distinguishing case. `= T` is a document title, so this is not a
    // whitespace-only document, and the earlier guard does not see it.
    let error = parse("= T\n\n\r\r").expect_err("a titled document can hang too");

    assert!(matches!(error, DomainError::ParseFailed { .. }));
}

#[test]
fn the_carriage_return_refusal_reports_where_it_found_it() {
    let error = parse("= T\n\n\r\r").expect_err("a titled document can hang too");

    let DomainError::ParseFailed { location, .. } = error else {
        panic!("expected a parse failure");
    };
    assert_eq!(
        location.line, 3,
        "the offending carriage return is on line 3"
    );
}

#[test]
fn windows_line_endings_are_not_refused() {
    // The rule counts carriage returns per line, so CRLF text has one to a
    // line and never trips it. Refusing every Windows document would be a far
    // worse defect than the hang.
    for source in [
        "= Title\r\n\r\nA paragraph.\r\n",
        "\r\n\r\n\r\n",
        "One line.\r\n",
        "* one\r\n* two\r\n",
    ] {
        assert!(
            parse(source).is_ok(),
            "this document must still parse: {source:?}"
        );
    }
}

#[test]
fn the_refusal_is_wider_than_the_defect_and_says_so() {
    // These three parse upstream, and are refused anyway. The pattern is
    // necessary for the hang but not sufficient, and the exact condition
    // depends on block structure in a way not worth encoding against a defect
    // that should be fixed upstream. Recorded as a test rather than left as a
    // surprise: if a later change narrows the guard, this is where it shows.
    for source in ["\r\ra", "a\r\r\r", "a\n\r\r"] {
        assert!(
            parse(source).is_err(),
            "the guard is deliberately conservative here: {source:?}"
        );
    }
}

#[test]
fn a_carriage_return_that_is_not_followed_by_whitespace_is_accepted() {
    // The refusal is bounded: only a carriage return followed by *whitespace*
    // other than a line feed is refused.
    for source in ["\ra", "a\rb", "\r", "line\r"] {
        assert!(
            parse(source).is_ok(),
            "this document must still parse: {source:?}"
        );
    }
}

#[test]
fn a_parser_sentinel_in_source_is_refused() {
    // `asciidoc-parser` brackets its cross-reference placeholders with U+E000
    // and U+E001, and its footnote markers with U+E002 and U+E003, on the
    // stated assumption that they "cannot collide with user text". A document
    // can type them. `"*\u{e000}<<>>"` makes the parser misread its own
    // output: a debug assertion in a debug build, a corrupt placeholder in the
    // rendered text otherwise.
    //
    // Found by the property test beside the inline decoder, which generates
    // private-use characters among ordinary content.
    for source in [
        "*\u{e000}<<>>",
        "text \u{e001} more",
        "\u{e002}",
        "a\u{e003}b",
    ] {
        let error = parse(source).expect_err("a parser sentinel must be refused");

        assert!(
            error.to_string().contains("asciidoc-parser"),
            "the message must say whose sentinel it is, got: {error}"
        );
    }
}

#[test]
fn a_character_reference_cannot_smuggle_a_reserved_character_past_the_guard() {
    // The guard reads the source, and a numeric character reference spells the
    // character in ASCII, so the guard cannot see it. The reference is resolved
    // later, by the decoder — which is why the decoder refuses the same two
    // ranges. Without that, `&#xE000;` put back exactly what the guard exists
    // to keep out: `"*&#xE000;<<>>"` reproduced the upstream defect in full.
    for source in [
        "&#xE000;text",
        "&#57344;text",
        "*&#xE000;<<>>",
        "&#xE003;",
        "&#xFDD0;text",
        "&#64976;text",
    ] {
        let outcome = parse(source).expect("a reference is content, not a refusal");

        let rendered = plain_text(&outcome);
        assert!(
            !rendered.chars().any(|character| {
                ('\u{e000}'..='\u{e003}').contains(&character)
                    || ('\u{fdd0}'..='\u{fdef}').contains(&character)
            }),
            "a reserved character reached the document from {source:?}: {rendered:?}"
        );
    }
}

#[test]
fn an_unresolvable_character_reference_is_left_as_the_author_wrote_it() {
    // Refusing to resolve it must not lose the text either: what the author
    // typed still reaches the page, as an ordinary run of characters.
    let outcome = parse("&#xE000;text").expect("accepted");

    assert!(
        plain_text(&outcome).contains("&#xE000;"),
        "the reference must survive as written, got: {:?}",
        plain_text(&outcome)
    );
}

#[test]
fn a_reference_resolving_to_an_ampersand_does_not_decode_twice() {
    // `&#38;` is an ampersand, and the four characters after it are `lt;`. The
    // author asked for the text `&lt;`, not for a less-than sign. The decoder
    // reads left to right and cannot tell a resolved `&` from one the parser
    // emitted, so the encoder writes `&amp;` instead of a bare ampersand.
    for source in ["a &#38;lt; b", "a &amp;lt; b", "a \\&lt; b"] {
        let outcome = parse(source).expect("accepted");

        assert_eq!(
            plain_text(&outcome).trim(),
            "a &lt; b",
            "the author asked for four characters, not for a sign: {source:?}"
        );
    }
}

#[test]
fn ordinary_character_references_still_resolve() {
    // The refusal is bounded to the two reserved ranges.
    let outcome = parse("An em dash &#8212; and a &#x2192; arrow.").expect("accepted");
    let rendered = plain_text(&outcome);

    assert!(
        rendered.contains('\u{2014}') && rendered.contains('\u{2192}'),
        "ordinary references must still resolve, got: {rendered:?}"
    );
}

#[test]
fn ordinary_private_use_characters_are_still_accepted() {
    // The refusal is bounded to the four the parser reserves. Private-use
    // characters are used in practice — icon fonts put glyphs there — so
    // refusing the whole area would break real documents.
    for source in ["icon \u{f8ff} here", "\u{e004}", "\u{efff}"] {
        assert!(
            parse(source).is_ok(),
            "this document must still parse: {source:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// A panic, rather than a hang.
//
// The cases above are all guards that read the source and decline before
// parsing. This last group is different in kind: `asciidoc-parser` 0.29.19
// *panics* on an inline `image:` or `icon:` macro with no target, and no rule
// about the source text can decide which documents do that, so the unwind is
// caught at the parse call instead. See the guard's own documentation.
// ---------------------------------------------------------------------------

/// The reproducer the `parse_plan_emit` fuzz target found, ten bytes as
/// libFuzzer minimised it.
///
/// Recorded as `fuzz/artifacts/parse_plan_emit/crash-50078a102232...`. The
/// trailing `\u{2}]` is incidental — the macro alone is enough, as the case
/// below shows.
const FUZZED_IMAGE_MACRO: &str = "image:[]\u{2}]";

#[test]
fn the_fuzzed_image_macro_is_refused_rather_than_crashing() {
    let error = parse(FUZZED_IMAGE_MACRO).expect_err("this input panics the parser");

    assert!(
        matches!(error, DomainError::ParseFailed { .. }),
        "expected a parse failure, got {error:?}"
    );
}

#[test]
fn an_image_macro_with_no_target_is_refused_however_it_is_written() {
    // Nothing here is adversarial. `image:[Figure 1]` is what an author writes
    // when they forget the filename, and before this guard it crashed the
    // renderer.
    for source in [
        "image:[]",
        "icon:[]",
        "image:[alt]",
        "icon:[x]",
        "See image:[Figure 1] here.",
        "`image:[]`",
        "image:[\n]",
    ] {
        let error = parse(source).expect_err("this input panics the parser");

        assert!(
            error.to_string().contains("image:"),
            "the message must name the construct that caused it, got: {error}"
        );
    }
}

#[test]
fn a_macro_assembled_out_of_an_attribute_is_refused_too() {
    // The case that decided the mechanism. Attribute references are
    // substituted *before* macros, so neither of these documents contains the
    // text `image:` in a form a guard reading the source could match, and both
    // panic. A rule about the source cannot be written; catching the unwind
    // needs no rule at all.
    for source in [
        ":foo: imag\n{foo}e:[]",
        ":foo: e:[]\nimag{foo}",
        ":foo: image:[]\n{foo}",
        ":foo: image:\n{foo}[]",
    ] {
        assert!(
            parse(source).is_err(),
            "a macro assembled after substitution must be refused too: {source:?}"
        );
    }
}

#[test]
fn the_panic_refusal_is_distinguishable_from_a_syntax_error() {
    let error = parse("image:[alt]").expect_err("refused");
    let message = error.to_string();

    assert!(
        message.contains("boundary.adoc"),
        "the error must name the document, got: {message}"
    );
    assert!(
        message.contains("asciidoc-parser"),
        "the message must say what failed, got: {message}"
    );
    assert!(
        message.contains("panicked"),
        "the message must say how it failed, got: {message}"
    );
}

#[test]
fn image_macros_that_name_a_target_are_unaffected() {
    // The guard catches a panic; it refuses nothing on its own. Every image
    // macro that does not trip the upstream defect parses exactly as it did,
    // and so does prose that merely contains the word.
    for source in [
        "image:diagram.png[Architecture]",
        "image:https://example.com/x.png[]",
        "icon:tip[]",
        "image::diagram.png[]",
        "click the icon: a small bell",
        "\\image:[]",
    ] {
        assert!(
            parse(source).is_ok(),
            "this document must still parse: {source:?}"
        );
    }
}

/// The second crash the same fuzz target found, twenty bytes as libFuzzer
/// produced it.
///
/// Recorded as `fuzz/artifacts/parse_plan_emit/crash-cd41702258f4...`. It
/// minimises to two consecutive block attribute lines where the first holds a
/// `%` option shorthand, whitespace, and another `%`.
const FUZZED_SHORTHAND_ATTRLIST: &str = "\\i\n[%%%%\t\t%%%f]\r\n[f]";

#[test]
fn a_shorthand_attrlist_that_trips_a_debug_assertion_does_not_escape() {
    // Unlike the image macro, this one is a `debug_assert!` in
    // `attributes/element_attribute.rs:509` — it fires in a debug build and is
    // compiled out of a release one, where the parser instead discards a
    // warning it says cannot happen. So the assertion here is about
    // containment in both profiles: the call returns rather than taking the
    // process with it, and *when* the assertion is compiled in, what it
    // returns is the refusal.
    for source in [
        FUZZED_SHORTHAND_ATTRLIST,
        "[%\t%f]\n[f]",
        "[% %]\n[f]",
        "[% \t%f]\n[f]",
    ] {
        let outcome = parse(source);

        if cfg!(debug_assertions) {
            assert!(
                outcome.is_err(),
                "the debug assertion must be contained as a refusal: {source:?}"
            );
        }
    }
}

#[test]
fn an_ordinary_shorthand_attrlist_is_unaffected() {
    // The neighbouring shapes, none of which trips it.
    for source in [
        "[%a%b]\np",
        "[%a b]\np",
        "[.role #id]\np",
        "[%\t%f]",
        "[%\t%f]\ntext",
    ] {
        assert!(
            parse(source).is_ok(),
            "this document must still parse: {source:?}"
        );
    }
}
