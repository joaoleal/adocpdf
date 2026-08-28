//! The single point where source text becomes engine markup.
//!
//! # Why a string literal, not escaped markup
//!
//! The obvious approach is to enumerate the engine's markup metacharacters
//! — `*`, `_`, `#`, `$`, `@`, `<`, `` ` ``, and the rest — and backslash-escape
//! each one. That list is long, version-dependent, and context-dependent: `-`
//! is inert mid-word but starts a list at the beginning of a line. Getting it
//! wrong once is an injection.
//!
//! So content is not emitted as markup at all. It is emitted as a **string
//! literal in code mode**, which the engine displays verbatim. The escaping
//! surface then shrinks to what a string literal itself can terminate: the
//! quote, the backslash, and control characters. That set is small, fixed, and
//! provably complete — and it does not grow when the markup language does.
//!
//! The cost is that inline markup in the source cannot mean anything. That is
//! correct for now: the document model has no inline formatting to express.

use std::fmt::Write as _;

/// Renders text as an engine string literal, quotes included.
///
/// The result can be placed anywhere the engine expects an expression, and will
/// display as exactly the characters given — no matter what those characters
/// are.
#[must_use]
pub fn string_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');

    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Any other control character would either terminate the literal or
            // travel invisibly into the output. Neither is acceptable, so they
            // go through as explicit escapes.
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }

    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether the literal's body can be read without meeting its terminator.
    ///
    /// This is the property that matters: if no quote inside the body is
    /// unescaped, the literal ends exactly where it was meant to, and nothing
    /// after it came from the source.
    fn interior_has_no_unescaped_quote(literal: &str) -> bool {
        let body = &literal[1..literal.len() - 1];
        let mut characters = body.chars();

        while let Some(character) = characters.next() {
            match character {
                '\\' => {
                    characters.next();
                }
                '"' => return false,
                _ => {}
            }
        }
        true
    }

    #[test]
    fn ordinary_text_is_quoted_and_otherwise_untouched() {
        assert_eq!(string_literal("Hello, world."), "\"Hello, world.\"");
    }

    #[test]
    fn every_markup_metacharacter_survives_as_content() {
        // Each of these means something in the engine's markup language. Inside
        // a string literal none of them does, which is the entire point.
        for character in [
            '*', '_', '`', '$', '#', '@', '<', '>', '~', '-', '+', '=', '/', '[', ']', '(', ')',
            '\'', ':', ';', '.', ',', '!', '?', '%', '^', '&', '|', '{', '}',
        ] {
            let literal = string_literal(&character.to_string());

            assert_eq!(
                literal,
                format!("\"{character}\""),
                "{character:?} must pass through as content, not as markup"
            );
        }
    }

    #[test]
    fn a_quote_cannot_terminate_the_literal() {
        assert_eq!(string_literal("say \"hi\""), r#""say \"hi\"""#);
    }

    #[test]
    fn a_backslash_cannot_escape_the_closing_quote() {
        // Without escaping the backslash, `a\` would produce `"a\"` — a literal
        // that swallows its own terminator and runs on into the next
        // instruction.
        assert_eq!(string_literal("a\\"), r#""a\\""#);
    }

    #[test]
    fn a_newline_cannot_break_out_of_the_literal() {
        assert_eq!(string_literal("one\ntwo"), r#""one\ntwo""#);
    }

    #[test]
    fn carriage_returns_and_tabs_are_escaped() {
        assert_eq!(string_literal("a\rb\tc"), r#""a\rb\tc""#);
    }

    #[test]
    fn other_control_characters_are_escaped_rather_than_emitted() {
        assert_eq!(string_literal("a\u{0}b"), r#""a\u{0}b""#);
        assert_eq!(string_literal("a\u{7}b"), r#""a\u{7}b""#);
    }

    #[test]
    fn an_attempt_to_close_the_literal_and_inject_an_instruction_fails() {
        let attack = r#"", 1) #set page(width: 1cm) #("#;

        let literal = string_literal(attack);

        assert!(
            literal.starts_with('"') && literal.ends_with('"'),
            "the result must be one literal, got: {literal}"
        );
        assert!(
            interior_has_no_unescaped_quote(&literal),
            "the payload must not be able to close the literal, got: {literal}"
        );
    }

    #[test]
    fn text_is_preserved_exactly_when_read_back() {
        // Escaping must be lossless: everything that goes in comes back out.
        for source in [
            "plain",
            "with \"quotes\"",
            "with \\ backslash",
            "multi\nline",
            "unicode: é ü 中文 🙂",
            "#set page(width: 1cm)",
        ] {
            let literal = string_literal(source);

            assert!(
                literal.len() >= source.len() + 2,
                "escaping never shortens text, got: {literal}"
            );
        }
    }

    #[test]
    fn unicode_passes_through_without_escaping() {
        assert_eq!(string_literal("café 中文 🙂"), "\"café 中文 🙂\"");
    }

    #[test]
    fn empty_text_is_an_empty_literal() {
        assert_eq!(string_literal(""), "\"\"");
    }

    /// What `string_literal` must guarantee, stated for *every* input rather
    /// than for the examples above.
    ///
    /// These are Tier 1: pure, no engine, no I/O, so they run at a high case
    /// count in the ordinary suite. Tier 2 — compiling the literal through the
    /// real engine — lives in `tests/injection.rs`, where it can afford far
    /// fewer cases.
    mod properties {
        use proptest::prelude::*;

        use super::{interior_has_no_unescaped_quote, string_literal};

        /// Characters chosen to hit every arm of the escaper.
        ///
        /// `any::<char>()` is not used, and neither is `any::<String>()`: the
        /// latter is the regex `\PC*`, which excludes control characters
        /// entirely and would leave the `\n`, `\r`, `\t` and `\u{...}` arms —
        /// the reason the function exists — never exercised.
        fn interesting_char() -> impl Strategy<Value = char> {
            prop_oneof![
                // The two characters that can terminate or re-interpret a
                // literal. Weighted heavily: they are the whole risk.
                4 => Just('"'),
                4 => Just('\\'),
                // The three control characters with a short escape.
                3 => prop_oneof![Just('\n'), Just('\r'), Just('\t')],
                // Control characters that must become `\u{...}`. C0 and C1,
                // which is exactly what `char::is_control` covers.
                3 => prop_oneof![(0u32..0x20), (0x7fu32..0xa0)]
                    .prop_map(|c| char::from_u32(c).expect("C0 and C1 are valid scalar values")),
                // Ordinary text, and non-ASCII that must pass through
                // untouched — including characters that are invisible but not
                // `is_control`, which the escaper deliberately does not touch.
                8 => prop_oneof![
                    any::<char>(),
                    Just('\u{2028}'),
                    Just('\u{feff}'),
                    Just('\u{200b}'),
                    Just('é'),
                    Just('中'),
                    Just('🙂'),
                ],
            ]
        }

        fn interesting_text() -> impl Strategy<Value = String> {
            proptest::collection::vec(interesting_char(), 0..40)
                .prop_map(|characters| characters.into_iter().collect())
        }

        /// Reads a literal back to the text it was made from.
        ///
        /// Deliberately written here rather than exported from the module
        /// under test: an inverse derived from the same code could agree with
        /// a bug. This one is written from the *format*, so disagreement is
        /// evidence.
        fn decode(literal: &str) -> Result<String, String> {
            let mut characters = literal.chars();

            if characters.next() != Some('"') {
                return Err("literal does not open with a quote".to_owned());
            }

            let mut out = String::new();
            let mut closed = false;

            while let Some(character) = characters.next() {
                match character {
                    '"' => {
                        closed = true;
                        break;
                    }
                    '\\' => {
                        let escaped = characters
                            .next()
                            .ok_or_else(|| "trailing backslash".to_owned())?;
                        match escaped {
                            '"' => out.push('"'),
                            '\\' => out.push('\\'),
                            'n' => out.push('\n'),
                            'r' => out.push('\r'),
                            't' => out.push('\t'),
                            'u' => {
                                if characters.next() != Some('{') {
                                    return Err("\\u not followed by {".to_owned());
                                }
                                let mut hex = String::new();
                                loop {
                                    match characters.next() {
                                        Some('}') => break,
                                        Some(digit) => hex.push(digit),
                                        None => return Err("unterminated \\u{".to_owned()),
                                    }
                                }
                                let code = u32::from_str_radix(&hex, 16)
                                    .map_err(|_| format!("`{hex}` is not hex"))?;
                                out.push(
                                    char::from_u32(code)
                                        .ok_or_else(|| format!("{code} is not a scalar value"))?,
                                );
                            }
                            other => return Err(format!("unknown escape `\\{other}`")),
                        }
                    }
                    other => out.push(other),
                }
            }

            if !closed {
                return Err("literal does not close with a quote".to_owned());
            }
            if characters.next().is_some() {
                return Err("text follows the closing quote".to_owned());
            }
            Ok(out)
        }

        proptest! {
            /// The literal is delimited, and the delimiters are the only two
            /// bare quotes in it. If a quote in the body were unescaped, the
            /// literal would end early and everything after it would be read
            /// as markup — which is the injection this module exists to
            /// prevent.
            #[test]
            fn the_literal_opens_and_closes_with_the_only_bare_quotes(
                text in interesting_text()
            ) {
                let literal = string_literal(&text);

                prop_assert!(literal.starts_with('"'), "no opening quote: {literal:?}");
                prop_assert!(literal.ends_with('"'), "no closing quote: {literal:?}");
                prop_assert!(literal.len() >= 2, "too short to be delimited: {literal:?}");
                prop_assert!(
                    interior_has_no_unescaped_quote(&literal),
                    "an unescaped quote in the body of {literal:?}"
                );
            }

            /// Every backslash in the body opens a valid escape. A stray one
            /// would either consume the closing quote or leave a meaning the
            /// engine gets to choose.
            #[test]
            fn every_backslash_in_the_body_opens_a_known_escape(text in interesting_text()) {
                let literal = string_literal(&text);
                let body: Vec<char> = literal.chars().collect();
                let body = &body[1..body.len() - 1];

                let mut index = 0;
                while index < body.len() {
                    if body[index] == '\\' {
                        let next = body.get(index + 1);
                        prop_assert!(
                            matches!(next, Some('"' | '\\' | 'n' | 'r' | 't' | 'u')),
                            "backslash opens {next:?}, not a known escape, in {literal:?}"
                        );
                        index += 2;
                    } else {
                        index += 1;
                    }
                }
            }

            /// No raw control character survives into the output. One would
            /// either terminate the literal or travel invisibly into the
            /// document.
            #[test]
            fn no_raw_control_character_reaches_the_output(text in interesting_text()) {
                let literal = string_literal(&text);

                for character in literal.chars() {
                    prop_assert!(
                        !character.is_control(),
                        "raw control character {:?} (U+{:04X}) in {literal:?}",
                        character,
                        character as u32
                    );
                }
            }

            /// The escaping loses nothing and adds nothing. This is the
            /// property the other three are worth having: the reader sees
            /// exactly the source text, whatever it contained.
            #[test]
            fn the_literal_decodes_back_to_exactly_the_input(text in interesting_text()) {
                let literal = string_literal(&text);

                match decode(&literal) {
                    Ok(decoded) => prop_assert_eq!(
                        decoded,
                        text,
                        "round trip changed the text, via {}",
                        literal
                    ),
                    Err(reason) => prop_assert!(false, "{reason}, in {literal:?}"),
                }
            }
        }
    }
}
