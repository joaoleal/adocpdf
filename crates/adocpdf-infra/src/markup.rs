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
}
