//! Arbitrary bytes as an AsciiDoc source, through the pure path.
//!
//! The claim under test is the one `SECURITY.md` makes: no input, however
//! malformed, causes a panic. The fuzzer is looking for a crash, not for a
//! wrong document — a wrong document is a rendering bug, and a panic on
//! untrusted input is a vulnerability.
//!
//! Deliberately absent from this target:
//!
//! - **Any filesystem call.** The parser runs in `SafeMode::Secure`, so a
//!   document cannot widen its own access, and nothing here opens, reads or
//!   writes a file. Fuzzing a path that touches disk would spend the budget on
//!   the operating system.
//! - **PDF layout.** `typst::compile` is orders of magnitude slower than the
//!   parse and the plan, and it is upstream code. Including it would cut the
//!   number of inputs explored per second to a fraction for coverage of
//!   somebody else's crate.
//!
//! The date is fixed rather than read from a clock, so a crash found today
//! reproduces tomorrow.

#![no_main]

use adocpdf_asciidoc::parser::AsciidocParser;
use adocpdf_core::theme::ThemeSet;
use adocpdf_domain::document_plan::plan_document;
use adocpdf_domain::ports::{Date, DocumentParser};
use adocpdf_typst::emitter::emit;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Non-UTF-8 input is rejected before the parser ever sees it, so feeding
    // it here would only measure how fast `from_utf8` fails.
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    let today = match Date::new(2026, 8, 16) {
        Ok(date) => date,
        Err(_) => return,
    };

    // An error is a legitimate outcome and not interesting. A panic is the
    // finding, and it needs no assertion: libFuzzer treats it as a crash.
    let Ok(outcome) = AsciidocParser.parse(source, "fuzz.adoc", today) else {
        return;
    };

    let Ok(plan) = plan_document(&outcome.document, &ThemeSet::default()) else {
        return;
    };

    let markup = emit(&plan);

    // The one property asserted rather than merely survived: every string
    // literal in the emitted markup is closed. If emission ever produced an
    // unterminated one, the engine would read the rest of the document as
    // markup — the injection this project is built to prevent.
    //
    // Counting quote characters would not do: an escaped quote inside a
    // literal is a quote character too, so `"a\"b"` holds three of them. The
    // scan below tracks whether it is inside a literal and skips the character
    // after a backslash, which is the only way the question has an answer.
    assert!(
        every_literal_is_closed(&markup),
        "emitted markup leaves a string literal unterminated"
    );
});

fn every_literal_is_closed(markup: &str) -> bool {
    let mut characters = markup.chars();
    let mut inside = false;

    while let Some(character) = characters.next() {
        match character {
            '\\' if inside => {
                // Whatever follows is escaped, including a quote, so it cannot
                // close the literal.
                if characters.next().is_none() {
                    return false;
                }
            }
            '"' => inside = !inside,
            _ => {}
        }
    }

    !inside
}
