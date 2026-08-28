//! Getting inline structure out of the parser, and back into the model.
//!
//! # Why a renderer rather than a parser
//!
//! `asciidoc-parser` does not expose an inline syntax tree. What it exposes is
//! [`InlineSubstitutionRenderer`], the same extension point its own HTML output
//! is built on: the parser walks the inline substitutions and calls the
//! renderer for each one, handing over content that has already been rendered
//! from the inside out. Implementing it is how a downstream format gets inline
//! structure without re-implementing the grammar.
//!
//! # Why the structure travels as characters
//!
//! The trait writes into a `&mut String`, so structure has to survive as
//! characters before it can become a tree. Two upstream facts decide which
//! characters:
//!
//! - Special characters **must** be rendered as the HTML entities `&lt;`,
//!   `&gt;` and `&amp;`. Later substitution steps match against those exact
//!   strings — the arrow replacements are `Regex::new(r"\\?-&gt;")` and
//!   siblings — so emitting anything else silently disables them. The decoder
//!   turns them back into characters as its last step.
//! - Passthrough content (`pass:[…]`, `+++…+++`) reaches the output verbatim,
//!   never passing through this renderer. Any marker an author can type is
//!   therefore forgeable.
//!
//! So the markers are Unicode noncharacters, and
//! `parser::refuse_input_that_could_forge_structure` refuses them at the door.
//! Unforgeable by construction, rather than by escaping.

use adocpdf_core::document::{InlineNode, InlineStyle, InlineText};
use asciidoc_parser::Parser;
use asciidoc_parser::attributes::Attrlist;
use asciidoc_parser::parser::{
    CalloutRenderParams, CharacterReplacementType, FootnoteRenderParams, IconRenderParams,
    ImageRenderParams, IndexTermRenderParams, InlineSubstitutionRenderer, LinkRenderParams,
    MenuRenderParams, QuoteScope, QuoteType, SpecialCharacter, XrefRenderParams,
};

/// Opens a span set in the style the marker names.
///
/// One marker per style, so that opening and closing need no argument encoding
/// between them. The range is reserved and refused in source, so these cannot
/// collide with content.
const STRONG_OPEN: char = '\u{fdd0}';
const EMPHASIS_OPEN: char = '\u{fdd1}';
const MONOSPACE_OPEN: char = '\u{fdd2}';
const SUPERSCRIPT_OPEN: char = '\u{fdd3}';
const SUBSCRIPT_OPEN: char = '\u{fdd4}';
const HIGHLIGHT_OPEN: char = '\u{fdd5}';
const UNDERLINE_OPEN: char = '\u{fdd6}';
const STRIKETHROUGH_OPEN: char = '\u{fdd7}';
const LARGER_OPEN: char = '\u{fddc}';
const SMALLER_OPEN: char = '\u{fddd}';

/// Closes the innermost open span.
const SPAN_CLOSE: char = '\u{fdd8}';

/// A break the author asked for.
const HARD_LINE_BREAK: char = '\u{fdd9}';

/// Opens a construct this renderer does not honour.
///
/// The construct's name follows, then [`UNSUPPORTED_SEPARATOR`], then whatever
/// text it carried, then [`SPAN_CLOSE`].
///
/// Recording this in the stream rather than in a side table is what keeps the
/// report attributable. All inline rendering happens inside `Parser::parse`,
/// in one pass over the whole document, so a renderer that pushed skips into a
/// list would produce a flat sequence with no way to say which block each came
/// from. Carried in-band, each skip is decoded with the block it sits in, and
/// the block's location is right there.
const UNSUPPORTED_OPEN: char = '\u{fdda}';

/// Separates an unsupported construct's name from the text it carried.
const UNSUPPORTED_SEPARATOR: char = '\u{fddb}';

/// The marker that opens a span in `style`.
const fn open_marker(style: InlineStyle) -> char {
    match style {
        InlineStyle::Strong => STRONG_OPEN,
        InlineStyle::Emphasis => EMPHASIS_OPEN,
        InlineStyle::Monospace => MONOSPACE_OPEN,
        InlineStyle::Superscript => SUPERSCRIPT_OPEN,
        InlineStyle::Subscript => SUBSCRIPT_OPEN,
        InlineStyle::Highlight => HIGHLIGHT_OPEN,
        InlineStyle::Underline => UNDERLINE_OPEN,
        InlineStyle::Strikethrough => STRIKETHROUGH_OPEN,
        InlineStyle::Larger => LARGER_OPEN,
        InlineStyle::Smaller => SMALLER_OPEN,
    }
}

/// The first character of the range reserved for structural markers.
pub const MARKER_RANGE_START: char = '\u{fdd0}';

/// The last character of the range reserved for structural markers.
pub const MARKER_RANGE_END: char = '\u{fdef}';

/// The first character `asciidoc-parser` 0.29.19 reserves for itself.
///
/// Upstream brackets its cross-reference placeholders with U+E000 and U+E001,
/// and its footnote markers with U+E002 and U+E003, on the stated assumption
/// that they "cannot collide with user text". A document can type them, so
/// they are refused alongside this crate's own markers.
pub const SENTINEL_RANGE_START: char = '\u{e000}';

/// The last character `asciidoc-parser` 0.29.19 reserves for itself.
pub const SENTINEL_RANGE_END: char = '\u{e003}';

/// Whether `character` falls in the range this crate reserves for markers.
#[must_use]
pub const fn is_reserved_marker(character: char) -> bool {
    (character as u32) >= (MARKER_RANGE_START as u32)
        && (character as u32) <= (MARKER_RANGE_END as u32)
}

/// Whether `character` is one the parser reserves for its own placeholders.
#[must_use]
pub const fn is_parser_sentinel(character: char) -> bool {
    (character as u32) >= (SENTINEL_RANGE_START as u32)
        && (character as u32) <= (SENTINEL_RANGE_END as u32)
}

/// Whether this is one of the markers the renderer itself emits.
///
/// The reserved range is wider than the set in use, so that adding a marker
/// later does not widen what source is refused.
#[must_use]
pub const fn is_emitted_marker(character: char) -> bool {
    matches!(
        character,
        STRONG_OPEN
            | EMPHASIS_OPEN
            | MONOSPACE_OPEN
            | SUPERSCRIPT_OPEN
            | SUBSCRIPT_OPEN
            | HIGHLIGHT_OPEN
            | UNDERLINE_OPEN
            | STRIKETHROUGH_OPEN
            | LARGER_OPEN
            | SMALLER_OPEN
            | SPAN_CLOSE
            | HARD_LINE_BREAK
            | UNSUPPORTED_OPEN
            | UNSUPPORTED_SEPARATOR
    )
}

/// Renders AsciiDoc's inline substitutions as structure this crate can decode.
#[derive(Debug, Default, Clone)]
pub struct TypesetRenderer;

impl TypesetRenderer {
    /// Creates a renderer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl InlineSubstitutionRenderer for TypesetRenderer {
    fn render_special_character(&self, type_: SpecialCharacter, dest: &mut String) {
        // The HTML entities, deliberately. Later substitution steps match
        // against these exact strings; a different spelling would disable them.
        dest.push_str(match type_ {
            SpecialCharacter::Lt => "&lt;",
            SpecialCharacter::Gt => "&gt;",
            SpecialCharacter::Ampersand => "&amp;",
        });
    }

    fn render_quoted_substitution(
        &self,
        type_: QuoteType,
        _scope: QuoteScope,
        attrlist: Option<Attrlist<'_>>,
        _id: Option<String>,
        body: &str,
        dest: &mut String,
    ) {
        // The quote's own presentation first, then the roles around it, so
        // that `[.underline]*bold*` reaches the page as both. Written into a
        // buffer rather than straight into `dest` because each role wraps
        // everything decided so far.
        let mut quoted = String::new();
        quote(type_, body, &mut quoted);

        // An id is deliberately dropped. It is only useful once something can
        // link to it, and nothing can yet; carrying it would be weight in the
        // model with no reader.
        for role in attrlist.as_ref().map(Attrlist::roles).unwrap_or_default() {
            let mut wrapped = String::new();
            match InlineStyle::from_role(role) {
                Some(style) => wrap(style, &quoted, &mut wrapped),
                // A role with no typographic meaning here is reported by name
                // and its text kept. A role is a stylesheet class by origin,
                // and this renderer has no stylesheet to consult.
                None => unsupported(&format!("role {role:?}"), &quoted, &mut wrapped),
            }
            quoted = wrapped;
        }

        dest.push_str(&quoted);
    }

    fn render_character_replacement(&self, type_: CharacterReplacementType, dest: &mut String) {
        // Real characters, not entities: only `&lt;`, `&gt;` and `&amp;` have
        // to survive as entities, because only those are matched by later
        // substitution steps.
        //
        // The zero-width spaces the HTML renderer appends to em dashes and
        // ellipses are omitted. They exist to give a browser a break
        // opportunity; the layout engine here has its own line breaker and
        // does not need a hint smuggled in as content.
        match type_ {
            CharacterReplacementType::Copyright => dest.push('\u{a9}'),
            CharacterReplacementType::Registered => dest.push('\u{ae}'),
            CharacterReplacementType::Trademark => dest.push('\u{2122}'),
            CharacterReplacementType::EmDashSurroundedBySpaces => {
                dest.push('\u{2009}');
                dest.push('\u{2014}');
                dest.push('\u{2009}');
            }
            CharacterReplacementType::EmDashWithoutSpace => dest.push('\u{2014}'),
            CharacterReplacementType::Ellipsis => dest.push('\u{2026}'),
            CharacterReplacementType::SingleLeftArrow => dest.push('\u{2190}'),
            CharacterReplacementType::DoubleLeftArrow => dest.push('\u{21d0}'),
            CharacterReplacementType::SingleRightArrow => dest.push('\u{2192}'),
            CharacterReplacementType::DoubleRightArrow => dest.push('\u{21d2}'),
            CharacterReplacementType::TypographicApostrophe => dest.push('\u{2019}'),
            CharacterReplacementType::CharacterReference(name) => {
                resolve_character_reference(&name, dest);
            }
        }
    }

    fn render_line_break(&self, dest: &mut String) {
        dest.push(HARD_LINE_BREAK);
    }

    // Everything below is a construct this tier does not honour. Each is
    // overridden all the same, and none of them may fall through to the
    // inherited default: those defaults emit HTML, so a method left
    // un-overridden puts `<a href=…>` or `<img src=…>` on a typeset page.
    //
    // Each keeps the text the construct carried, so nothing an author wrote is
    // lost, and names itself so the report can say what was skipped.

    fn render_image(&self, params: &ImageRenderParams<'_>, dest: &mut String) {
        unsupported("inline image", &params.alt, dest);
    }

    fn image_uri(&self, target_image_path: &str, _parser: &Parser, _key: Option<&str>) -> String {
        // Overridden to keep the inherited implementation away from the disk:
        // it resolves the path and may embed the file's bytes as a data URI.
        // Reading a file the document names is the `project-sandbox`
        // requirement's business, not an inline renderer's.
        target_image_path.to_owned()
    }

    fn render_icon(&self, params: &IconRenderParams<'_>, dest: &mut String) {
        unsupported("icon", &params.alt, dest);
    }

    fn icon_uri(&self, name: &str, _attrlist: &Attrlist<'_>, _parser: &Parser) -> String {
        // Same reasoning as `image_uri`: the default derives a path and reads.
        name.to_owned()
    }

    fn render_link(&self, params: &LinkRenderParams<'_>, dest: &mut String) {
        // The link text is what a reader sees; where it pointed is tier 3.
        let text = if params.link_text.is_empty() {
            params.target.clone()
        } else {
            params.link_text.clone()
        };
        unsupported("link", &text, dest);
    }

    fn render_anchor(&self, id: &str, reftext: Option<String>, dest: &mut String) {
        unsupported("anchor", &reftext.unwrap_or_else(|| id.to_owned()), dest);
    }

    fn render_xref(&self, params: &XrefRenderParams<'_>, dest: &mut String) {
        let text = params.provided_text.unwrap_or(params.target);
        unsupported("cross-reference", text, dest);
    }

    fn render_callout(&self, params: &CalloutRenderParams<'_>, dest: &mut String) {
        unsupported("callout", params.number, dest);
    }

    fn render_index_term(&self, params: &IndexTermRenderParams<'_>, dest: &mut String) {
        // An invisible index term carries no text; the construct is still
        // reported, so its absence from the page is not a silent one.
        unsupported("index term", params.visible_term.unwrap_or_default(), dest);
    }

    fn render_button(&self, text: &str, dest: &mut String) {
        unsupported("button", text, dest);
    }

    fn render_keyboard(&self, keys: &[String], dest: &mut String) {
        unsupported("keyboard shortcut", &keys.join("+"), dest);
    }

    fn render_menu(&self, params: &MenuRenderParams<'_>, dest: &mut String) {
        let mut path = vec![params.menu.to_owned()];
        path.extend(params.submenus.iter().cloned());
        if let Some(item) = params.menuitem {
            path.push(item.to_owned());
        }
        // The separator is a real character, not `>`: a rendered page shows
        // "File \u{25b8} Save", and it keeps the leak check for `<` and `>`
        // honest about what came from an un-overridden default.
        unsupported("menu selection", &path.join(" \u{25b8} "), dest);
    }

    fn render_footnote(&self, params: &FootnoteRenderParams<'_>, dest: &mut String) {
        // The footnote's body is not reachable from here. `params.text` is
        // documented upstream as carrying the unresolved ID and being "ignored
        // in the other cases"; a resolved footnote's content lives in the
        // document's catalog, which the HTML converter emits separately at the
        // foot of the page. So a defining occurrence can only keep its marker.
        let text = params
            .index
            .map_or_else(|| params.text.to_owned(), |index| format!("[{index}]"));
        unsupported("footnote", &text, dest);
    }
}

/// Writes `body` under the presentation the quote syntax asked for.
///
/// Separate from the roles that may wrap it: the quote syntax and the
/// attribute list are two independent asks, and `[.underline]*bold*` is
/// both.
fn quote(type_: QuoteType, body: &str, dest: &mut String) {
    // `body` has already been rendered by this same renderer, so nesting
    // composes without any work here.
    match type_ {
        QuoteType::Strong => wrap(InlineStyle::Strong, body, dest),
        QuoteType::Emphasis => wrap(InlineStyle::Emphasis, body, dest),
        QuoteType::Monospaced => wrap(InlineStyle::Monospace, body, dest),
        QuoteType::Superscript => wrap(InlineStyle::Superscript, body, dest),
        QuoteType::Subscript => wrap(InlineStyle::Subscript, body, dest),
        QuoteType::Mark => wrap(InlineStyle::Highlight, body, dest),

        // Curved quotation marks are typography, not a presentation, so
        // they are characters rather than a span.
        QuoteType::DoubleQuote => {
            dest.push('\u{201c}');
            dest.push_str(body);
            dest.push('\u{201d}');
        }
        QuoteType::SingleQuote => {
            dest.push('\u{2018}');
            dest.push_str(body);
            dest.push('\u{2019}');
        }

        // A span with no presentation of its own, and mathematical
        // notation, which is tier 5. Both keep their text and add no
        // structure: for the first that is the whole meaning, and for the
        // second it is what stops the notation being lost.
        QuoteType::Unquoted | QuoteType::AsciiMath | QuoteType::LatexMath => {
            dest.push_str(body);
        }
    }
}

/// Writes an unsupported construct: its name, and the text it carried.
///
/// The name is usually from this crate's own closed vocabulary. A role is the
/// exception — an author names it — and that is safe for the same reason the
/// markers themselves are: a role name comes from the source, and
/// `parser::refuse_input_that_could_forge_structure` has already refused every
/// character in the reserved range, so a name cannot hold a marker or the
/// separator that ends it. The text is arbitrary content and is written as-is;
/// the decoder treats it as text and nothing else.
fn unsupported(construct: &str, text: &str, dest: &mut String) {
    dest.push(UNSUPPORTED_OPEN);
    dest.push_str(construct);
    dest.push(UNSUPPORTED_SEPARATOR);
    dest.push_str(text);
    dest.push(SPAN_CLOSE);
}

/// Writes `body` wrapped in the markers for `style`.
fn wrap(style: InlineStyle, body: &str, dest: &mut String) {
    dest.push(open_marker(style));
    dest.push_str(body);
    dest.push(SPAN_CLOSE);
}

/// Writes the character a reference names, or the reference as written.
///
/// Numeric references are resolved arithmetically, and the five references XML
/// itself defines are resolved from a table. A named reference outside that set
/// is left exactly as the author wrote it: resolving the rest would mean
/// embedding the whole HTML entity table for a construct the specification does
/// not require, and silently dropping it would lose text.
fn resolve_character_reference(name: &str, dest: &mut String) {
    let resolved = match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => numeric_character_reference(name),
    };

    if let Some(character) = resolved {
        // An ampersand is written back as `&amp;` rather than as itself. The
        // decoder reads this stream left to right and cannot tell a character
        // this function resolved from one the parser emitted, so a bare `&`
        // here would join whatever followed it: `&#38;lt;` would reach the page
        // as `<` when the author asked for the four characters `&lt;`.
        if character == '&' {
            dest.push_str("&amp;");
        } else {
            dest.push(character);
        }
    } else {
        dest.push('&');
        dest.push_str(name);
        dest.push(';');
    }
}

/// Resolves `#8212` or `#x2014` to the character it names.
fn numeric_character_reference(name: &str) -> Option<char> {
    let digits = name.strip_prefix('#')?;

    let code_point = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse::<u32>().ok()?,
    };

    let character = char::from_u32(code_point)?;

    // A reference naming a reserved character is left unresolved. The guard at
    // the parse boundary refuses both ranges, but it cannot see this one: the
    // source spells the character in ASCII, and the reference is resolved here,
    // after the guard has run. Resolving it would put back exactly what the
    // guard was written to keep out — a marker this crate reads as structure,
    // or a sentinel the parser reads as its own placeholder.
    if is_reserved_marker(character) || is_parser_sentinel(character) {
        return None;
    }

    Some(character)
}

/// What a decoded run of inline content carried.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecodedInline {
    /// The content, ready for the model.
    pub text: InlineText,
    /// The constructs that were not honoured, in document order.
    ///
    /// Names only. Where in the source each sat is the caller's to supply:
    /// the upstream renderer is handed no span, so the enclosing block's
    /// location is the finest granularity available.
    pub unsupported: Vec<String>,
}

/// The style a marker opens, if it opens one.
const fn style_of(marker: char) -> Option<InlineStyle> {
    match marker {
        STRONG_OPEN => Some(InlineStyle::Strong),
        EMPHASIS_OPEN => Some(InlineStyle::Emphasis),
        MONOSPACE_OPEN => Some(InlineStyle::Monospace),
        SUPERSCRIPT_OPEN => Some(InlineStyle::Superscript),
        SUBSCRIPT_OPEN => Some(InlineStyle::Subscript),
        HIGHLIGHT_OPEN => Some(InlineStyle::Highlight),
        UNDERLINE_OPEN => Some(InlineStyle::Underline),
        STRIKETHROUGH_OPEN => Some(InlineStyle::Strikethrough),
        LARGER_OPEN => Some(InlineStyle::Larger),
        SMALLER_OPEN => Some(InlineStyle::Smaller),
        _ => None,
    }
}

/// What an open marker started.
#[derive(Debug)]
enum Frame {
    /// A presentation, which becomes a styled node when it closes.
    Styled(InlineStyle),
    /// An unsupported construct, whose content is kept but whose presentation
    /// is discarded when it closes.
    Unsupported(String),
}

/// What a newline inside decoded text means for the block being decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftBreaks {
    /// The newline is where the author's editor wrapped the source, and the
    /// layout engine decides where the line really ends. This is what AsciiDoc
    /// means by a paragraph, and it is what almost every block wants.
    Fill,
    /// The newline is a line ending the author chose, and the reader must see
    /// it. A verse is the case: its shape is the content.
    Keep,
    /// The text is not laid out at all and must be handed back exactly as it
    /// arrived. Verbatim content is the case: it is read for its characters,
    /// and a newline in it is one of them.
    Preserve,
}

/// Turns the encoded stream back into content and a list of what was skipped,
/// filling paragraphs.
///
/// The entities are decoded last and in one left-to-right pass, never by
/// successive whole-string replacements: `&amp;lt;` must become the literal
/// text `&lt;`, and replacing `&lt;` first would turn it into `<`.
#[must_use]
pub fn decode(encoded: &str) -> DecodedInline {
    decode_with(encoded, SoftBreaks::Fill)
}

/// The same, for content handed back exactly as it arrived.
///
/// Verbatim content only. It is decoded so that markers and entities the parser
/// emitted are resolved, and then nothing else is done to it: every space and
/// newline in a listing block is one the author typed.
#[must_use]
pub fn decode_preserving_text(encoded: &str) -> DecodedInline {
    decode_with(encoded, SoftBreaks::Preserve)
}

/// The same, for a block whose line endings the author chose.
///
/// The distinction is made here, where the caller knows what kind of block it
/// is reading, rather than downstream where only the text is left. A verse's
/// line endings become [`InlineNode::LineBreak`] — which is what they are —
/// instead of relying on a newline surviving as a newline all the way to the
/// page, which is how verse used to work and why filling paragraphs would
/// otherwise have silently reflowed every verse in every document.
#[must_use]
pub fn decode_keeping_line_breaks(encoded: &str) -> DecodedInline {
    decode_with(encoded, SoftBreaks::Keep)
}

fn decode_with(encoded: &str, soft_breaks: SoftBreaks) -> DecodedInline {
    let mut unsupported = Vec::new();
    // A stack of (what opened it, the nodes gathered inside it). The bottom
    // frame is the content itself and is never popped.
    let mut stack: Vec<(Option<Frame>, Vec<InlineNode>)> = vec![(None, Vec::new())];
    let mut pending = String::new();
    let mut characters = encoded.chars().peekable();

    while let Some(character) = characters.next() {
        if let Some(style) = style_of(character) {
            flush(&mut pending, &mut stack, soft_breaks);
            stack.push((Some(Frame::Styled(style)), Vec::new()));
        } else if character == UNSUPPORTED_OPEN {
            flush(&mut pending, &mut stack, soft_breaks);
            let mut name = String::new();
            for next in characters.by_ref() {
                if next == UNSUPPORTED_SEPARATOR {
                    break;
                }
                name.push(next);
            }
            stack.push((Some(Frame::Unsupported(name)), Vec::new()));
        } else if character == SPAN_CLOSE {
            flush(&mut pending, &mut stack, soft_breaks);
            close(&mut stack, &mut unsupported);
        } else if character == HARD_LINE_BREAK {
            flush(&mut pending, &mut stack, soft_breaks);
            push_node(&mut stack, InlineNode::LineBreak);
        } else if character == '&' {
            match decode_entity(&mut characters) {
                Some(decoded) => pending.push(decoded),
                None => pending.push('&'),
            }
        } else {
            pending.push(character);
        }
    }

    flush(&mut pending, &mut stack, soft_breaks);
    // An unclosed span is not an error: its content is still content. Closing
    // what is left keeps the text rather than discarding a whole subtree over
    // a marker the parser did not balance.
    while stack.len() > 1 {
        close(&mut stack, &mut unsupported);
    }

    let mut nodes = stack.pop().map(|(_, nodes)| nodes).unwrap_or_default();
    if soft_breaks != SoftBreaks::Preserve {
        tidy_around_breaks(&mut nodes);
    }

    DecodedInline {
        text: InlineText::from_nodes(nodes),
        unsupported,
    }
}

/// Reads the rest of an entity, returning the character it names.
///
/// Only the three the renderer emits are recognised. Anything else is left
/// alone, so an `&` the author wrote survives as an `&`.
fn decode_entity(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<char> {
    // Peeking a fixed number of characters is enough: the longest is `amp;`.
    let rest: String = characters.clone().take(4).collect();

    let (decoded, length) = if rest.starts_with("lt;") {
        ('<', 3)
    } else if rest.starts_with("gt;") {
        ('>', 3)
    } else if rest.starts_with("amp;") {
        ('&', 4)
    } else {
        return None;
    };

    for _ in 0..length {
        characters.next();
    }
    Some(decoded)
}

/// Removes the space a collapsed newline leaves beside an explicit break.
///
/// `line one +\nline two` holds both kinds of ending at once: the marker asks
/// for a break, and the newline after it is only where the source wrapped.
/// Filling turns that newline into a space, which would set the next line one
/// space in from the margin — a break followed by an indent nobody asked for.
///
/// Only spaces and tabs are trimmed, and only where they touch a break: the
/// whitespace elsewhere in a line is the author's.
fn tidy_around_breaks(nodes: &mut Vec<InlineNode>) {
    for node in nodes.iter_mut() {
        if let InlineNode::Styled { children, .. } = node {
            tidy_around_breaks(children);
        }
    }

    let breaks: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node, InlineNode::LineBreak))
        .map(|(index, _)| index)
        .collect();

    for index in breaks {
        if let Some(InlineNode::Text(before)) = index.checked_sub(1).and_then(|i| nodes.get_mut(i))
        {
            while before.ends_with([' ', '\t']) {
                before.pop();
            }
        }
        if let Some(InlineNode::Text(after)) = nodes.get_mut(index + 1) {
            let trimmed = after.trim_start_matches([' ', '\t']).to_owned();
            *after = trimmed;
        }
    }

    nodes.retain(|node| !matches!(node, InlineNode::Text(text) if text.is_empty()));
}

/// Moves gathered text into the frame it belongs to, honouring `soft_breaks`.
fn flush(
    pending: &mut String,
    stack: &mut [(Option<Frame>, Vec<InlineNode>)],
    soft_breaks: SoftBreaks,
) {
    if pending.is_empty() {
        return;
    }
    let gathered = std::mem::take(pending);
    let Some((_, nodes)) = stack.last_mut() else {
        return;
    };

    match soft_breaks {
        SoftBreaks::Fill => nodes.push(InlineNode::Text(fill(&gathered))),
        SoftBreaks::Keep => push_lines(&gathered, nodes),
        SoftBreaks::Preserve => nodes.push(InlineNode::Text(gathered)),
    }
}

/// Replaces every run of whitespace holding a newline with a single space.
///
/// A newline inside a paragraph is where the author's editor wrapped the
/// source, not a break the reader should see, and the layout engine is given a
/// string in which a newline would be a mandatory break. Collapsing the
/// surrounding horizontal whitespace with it matters: a source line ending in a
/// space would otherwise become two spaces at the join.
fn fill(text: &str) -> String {
    if !text.contains('\n') {
        return text.to_owned();
    }

    let mut out = String::with_capacity(text.len());
    let mut run = String::new();

    for character in text.chars() {
        if character.is_whitespace() {
            run.push(character);
            continue;
        }
        out.push_str(&collapsed(&run));
        run.clear();
        out.push(character);
    }
    out.push_str(&collapsed(&run));

    out
}

/// One run of whitespace, as it should be written back.
///
/// A run holding a newline becomes a single space; one that does not is left
/// exactly as the author typed it, because the spaces inside a line are theirs.
fn collapsed(run: &str) -> String {
    if run.contains('\n') {
        " ".to_owned()
    } else {
        run.to_owned()
    }
}

/// Pushes `text` with each of its line endings as an explicit break.
///
/// A trailing newline adds no break: it ends the last line rather than starting
/// an empty one, and a break there would set a blank line the author did not
/// write.
fn push_lines(text: &str, nodes: &mut Vec<InlineNode>) {
    let mut lines = text.split('\n').peekable();

    while let Some(line) = lines.next() {
        if !line.is_empty() {
            nodes.push(InlineNode::Text(line.to_owned()));
        }
        if lines.peek().is_some() {
            nodes.push(InlineNode::LineBreak);
        }
    }

    // `split` on a trailing newline yields a final empty piece, which pushed a
    // break above. Take it back off: nothing follows it.
    if text.ends_with('\n') {
        nodes.pop();
    }
}

/// Appends a node to the innermost frame.
fn push_node(stack: &mut [(Option<Frame>, Vec<InlineNode>)], node: InlineNode) {
    if let Some((_, nodes)) = stack.last_mut() {
        nodes.push(node);
    }
}

/// Closes the innermost frame, folding it into its parent.
fn close(stack: &mut Vec<(Option<Frame>, Vec<InlineNode>)>, unsupported: &mut Vec<String>) {
    // Never pop the bottom frame: a stray close marker in content would
    // otherwise leave nowhere to put the rest of the document.
    if stack.len() <= 1 {
        return;
    }

    let Some((frame, children)) = stack.pop() else {
        return;
    };

    match frame {
        Some(Frame::Styled(style)) => {
            push_node(stack, InlineNode::styled(style, children));
        }
        Some(Frame::Unsupported(name)) => {
            // The text is kept and the presentation is not: an unsupported
            // construct contributes its content to the page as ordinary text.
            unsupported.push(name);
            if let Some((_, nodes)) = stack.last_mut() {
                nodes.extend(children);
            }
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asciidoc_parser::Parser;
    use asciidoc_parser::blocks::{Block, FindBlocks};

    /// The encoded form of a document's first paragraph.
    ///
    /// This goes through the real parser rather than calling the trait methods
    /// directly, because what is under test is the agreement between this
    /// renderer and the substitution steps that drive it.
    pub(super) fn encode_document(source: &str) -> String {
        let mut parser =
            Parser::default().with_inline_substitution_renderer(TypesetRenderer::new());
        let document = parser.parse(source);

        for block in document.child_blocks() {
            if let Block::Simple(simple) = block {
                return simple.content().rendered().to_owned();
            }
        }
        panic!("the source has no paragraph: {source:?}");
    }

    /// Every run of rendered inline content in a document, joined.
    ///
    /// Unlike [`encode_document`] this tolerates a source that is not a
    /// paragraph at all — a property test generates blockquotes, lists and
    /// headings whether or not it means to, and refusing them would test only
    /// the inputs that happened to parse the expected way.
    pub(super) fn encode_all(source: &str) -> String {
        fn walk(block: &Block<'_>, out: &mut String) {
            match block {
                Block::Simple(simple) => out.push_str(simple.content().rendered()),
                Block::Section(section) => {
                    out.push_str(section.section_title());
                    for child in section.child_blocks() {
                        walk(child, out);
                    }
                }
                other => {
                    for child in other.child_blocks() {
                        walk(child, out);
                    }
                }
            }
        }

        let mut parser =
            Parser::default().with_inline_substitution_renderer(TypesetRenderer::new());
        let document = parser.parse(source);

        let mut out = String::new();
        for block in document.child_blocks() {
            walk(block, &mut out);
        }
        out
    }

    /// The same, with markers written as names so a failure is readable.
    fn readable(encoded: &str) -> String {
        encoded
            .chars()
            .map(|character| match character {
                STRONG_OPEN => "<strong>".to_owned(),
                EMPHASIS_OPEN => "<em>".to_owned(),
                MONOSPACE_OPEN => "<mono>".to_owned(),
                SUPERSCRIPT_OPEN => "<sup>".to_owned(),
                SUBSCRIPT_OPEN => "<sub>".to_owned(),
                HIGHLIGHT_OPEN => "<mark>".to_owned(),
                SPAN_CLOSE => "</>".to_owned(),
                UNSUPPORTED_OPEN => "<skip ".to_owned(),
                UNSUPPORTED_SEPARATOR => ":".to_owned(),
                HARD_LINE_BREAK => "<br>".to_owned(),
                other => other.to_string(),
            })
            .collect()
    }

    #[test]
    fn a_role_with_typographic_meaning_becomes_the_style_it_names() {
        for (source, expected) in [
            ("[.underline]#x#", InlineStyle::Underline),
            ("[.line-through]#x#", InlineStyle::Strikethrough),
            ("[.big]#x#", InlineStyle::Larger),
            ("[.small]#x#", InlineStyle::Smaller),
        ] {
            let decoded = decode(&encode_document(source));

            assert_eq!(
                decoded.text.nodes(),
                [InlineNode::styled(expected, vec![InlineNode::text("x")])],
                "for {source}"
            );
            assert!(decoded.unsupported.is_empty(), "for {source}");
        }
    }

    #[test]
    fn a_role_wraps_the_presentation_the_quote_syntax_asked_for() {
        let decoded = decode(&encode_document("[.underline]*bold*"));

        assert_eq!(
            decoded.text.nodes(),
            [InlineNode::styled(
                InlineStyle::Underline,
                vec![InlineNode::styled(
                    InlineStyle::Strong,
                    vec![InlineNode::text("bold")]
                )]
            )],
            "a role and the quote syntax are two independent asks"
        );
    }

    #[test]
    fn several_roles_on_one_span_all_apply() {
        let decoded = decode(&encode_document("[.underline.small]#x#"));

        assert_eq!(decoded.text.plain_text(), "x");

        let mut styles = Vec::new();
        let mut nodes = decoded.text.nodes().to_vec();
        while let [InlineNode::Styled { style, children }] = nodes.as_slice() {
            styles.push(*style);
            nodes = children.clone();
        }

        assert_eq!(styles, [InlineStyle::Smaller, InlineStyle::Underline]);
    }

    #[test]
    fn a_role_with_no_typographic_meaning_is_named_and_its_text_kept() {
        let decoded = decode(&encode_document("[.warning]#danger#"));

        assert_eq!(
            decoded.text.nodes(),
            [InlineNode::text("danger")],
            "the text is set no differently from the body text around it"
        );
        assert_eq!(decoded.unsupported, [r#"role "warning""#]);
    }

    #[test]
    fn a_wrapped_paragraph_carries_no_newline_into_the_model() {
        let decoded = decode(&encode_document("Alpha alpha\nbeta beta\ngamma.\n"));

        let text = decoded.text.plain_text();

        assert!(
            !text.contains('\n'),
            "a soft wrap is the editor's, not the author's, got: {text:?}"
        );
        assert_eq!(text.trim(), "Alpha alpha beta beta gamma.");
    }

    #[test]
    fn a_wrapped_line_ending_in_a_space_does_not_produce_two() {
        // The join is where a naive collapse goes wrong: the space before the
        // newline and the newline itself are one run of whitespace, not two.
        let decoded = decode(&encode_document("Alpha alpha \nbeta beta.\n"));

        assert!(
            !decoded.text.plain_text().contains("  "),
            "got: {:?}",
            decoded.text.plain_text()
        );
    }

    #[test]
    fn the_spaces_inside_a_line_are_left_alone() {
        // Only whitespace holding a newline is collapsed. What the author typed
        // within a line is theirs, and a renderer that tidied it would be
        // making an editorial decision it was not asked to make.
        let filled = fill("two  spaces\nand\ttabs");

        assert_eq!(filled, "two  spaces and\ttabs");
    }

    #[test]
    fn a_verse_keeps_its_line_endings_as_breaks() {
        let decoded = decode_keeping_line_breaks("first\nsecond\nthird");

        let breaks = decoded
            .text
            .nodes()
            .iter()
            .filter(|node| matches!(node, InlineNode::LineBreak))
            .count();

        assert_eq!(breaks, 2, "three lines are separated by two breaks");
    }

    #[test]
    fn a_trailing_newline_does_not_add_an_empty_line() {
        let decoded = decode_keeping_line_breaks("first\nsecond\n");

        assert!(
            !matches!(decoded.text.nodes().last(), Some(InlineNode::LineBreak)),
            "a break at the end would set a blank line, got: {:?}",
            decoded.text.nodes()
        );
    }
    #[test]
    fn each_style_gets_its_own_marker() {
        for (source, expected) in [
            ("a *b* c", "a <strong>b</> c"),
            ("a _b_ c", "a <em>b</> c"),
            ("a `b` c", "a <mono>b</> c"),
            ("a ^b^ c", "a <sup>b</> c"),
            ("a ~b~ c", "a <sub>b</> c"),
            ("a #b# c", "a <mark>b</> c"),
        ] {
            assert_eq!(
                readable(&encode_document(source)),
                expected,
                "for {source:?}"
            );
        }
    }

    #[test]
    fn nested_formatting_nests_its_markers() {
        assert_eq!(
            readable(&encode_document("*bold _and italic_*")),
            "<strong>bold <em>and italic</></>",
            "the parser renders inner content first, so nesting composes on its own"
        );
    }

    #[test]
    fn unconstrained_formatting_within_a_word_is_marked_too() {
        assert_eq!(
            readable(&encode_document("char**act**ers")),
            "char<strong>act</>ers"
        );
    }

    #[test]
    fn special_characters_stay_as_html_entities() {
        // Not a style choice: `substitution_step.rs` matches these exact
        // strings when it applies replacements and callouts.
        assert_eq!(encode_document("a < b > c & d"), "a &lt; b &gt; c &amp; d");
    }

    #[test]
    fn entity_spelling_keeps_the_arrow_replacements_working() {
        assert_eq!(
            encode_document("a -> b => c <- d <= e"),
            "a \u{2192} b \u{21d2} c \u{2190} d \u{21d0} e",
            "arrows are matched against `-&gt;` and siblings, so the entity spelling is \
             what makes them fire at all"
        );
    }

    #[test]
    fn typographic_replacements_become_real_characters() {
        assert_eq!(encode_document("(C) (R) (TM)"), "\u{a9} \u{ae} \u{2122}");
        // The replacement consumes the spaces around the dash and puts thin
        // spaces in their place, which is what the typography calls for.
        assert_eq!(encode_document("a -- b"), "a\u{2009}\u{2014}\u{2009}b");
        assert_eq!(encode_document("wait..."), "wait\u{2026}");
        assert_eq!(encode_document("it's"), "it\u{2019}s");
    }

    #[test]
    fn curved_quotation_marks_are_characters_not_a_style() {
        assert_eq!(encode_document("\"`quoted`\""), "\u{201c}quoted\u{201d}");
        assert_eq!(encode_document("'`quoted`'"), "\u{2018}quoted\u{2019}");
    }

    #[test]
    fn a_hard_line_break_becomes_a_marker() {
        assert_eq!(
            readable(&encode_document("line one +\nline two")),
            "line one<br>\nline two"
        );
    }

    #[test]
    fn an_escaped_delimiter_produces_no_span() {
        assert_eq!(
            readable(&encode_document(r"a \*b* c")),
            "a *b* c",
            "escaping is the author declining the formatting, not a construct to represent"
        );
    }

    #[test]
    fn numeric_character_references_resolve() {
        let mut out = String::new();
        resolve_character_reference("#8212", &mut out);
        resolve_character_reference("#x2014", &mut out);
        assert_eq!(out, "\u{2014}\u{2014}");
    }

    #[test]
    fn an_unknown_named_reference_is_left_as_written() {
        let mut out = String::new();
        resolve_character_reference("hellip", &mut out);
        assert_eq!(
            out, "&hellip;",
            "resolving every named entity would mean embedding the whole table; dropping it \
             would lose text"
        );
    }

    #[test]
    fn a_numeric_reference_cannot_name_a_marker() {
        let mut out = String::new();
        resolve_character_reference("#xFDD0", &mut out);
        assert_eq!(
            out, "&#xFDD0;",
            "the source guard cannot see this one, because the source spells it in ASCII"
        );
    }

    /// Every construct the renderer must override, with a source that triggers
    /// it and the text a reader should still see.
    fn unsupported_constructs() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("link", "See https://example.com[the site].", "the site"),
            ("link", "See link:page.html[the page].", "the page"),
            ("inline image", "An image:x.png[picture] here.", "picture"),
            ("icon", ":icons: font\n\nAn icon:heart[] here.", ""),
            ("footnote", "Text.footnote:[a note]", "[1]"),
            (
                "cross-reference",
                "See <<some-id,the section>>.",
                "the section",
            ),
            ("anchor", "[[marked]]Text.", "marked"),
            ("button", ":experimental:\n\nPress btn:[OK].", "OK"),
            (
                "keyboard shortcut",
                ":experimental:\n\nPress kbd:[Ctrl+C].",
                "Ctrl+C",
            ),
            (
                "menu selection",
                ":experimental:\n\nPick menu:File[Save].",
                "File \u{25b8} Save",
            ),
            ("index term", "Text ((a term)) here.", "a term"),
        ]
    }

    #[test]
    fn no_inherited_html_reaches_the_output() {
        // The defaults emit real HTML — `<a href=…>`, `<img src=…>`,
        // `<sup class="footnote">`. A method left un-overridden shows up here.
        for (_, source, _) in unsupported_constructs() {
            let encoded = encode_document(source);

            assert!(
                !encoded.contains('<'),
                "markup leaked from an un-overridden method for {source:?}: {}",
                readable(&encoded)
            );
        }
    }

    #[test]
    fn every_unsupported_construct_names_itself() {
        for (construct, source, _) in unsupported_constructs() {
            let encoded = encode_document(source);

            assert!(
                encoded.contains(&format!(
                    "{UNSUPPORTED_OPEN}{construct}{UNSUPPORTED_SEPARATOR}"
                )),
                "expected {source:?} to report {construct:?}, got: {}",
                readable(&encoded)
            );
        }
    }

    #[test]
    fn an_unsupported_construct_keeps_the_text_it_carried() {
        for (construct, source, text) in unsupported_constructs() {
            if text.is_empty() {
                continue;
            }
            let encoded = encode_document(source);

            assert!(
                encoded.contains(text),
                "{construct} must keep {text:?} for the reader, got: {}",
                readable(&encoded)
            );
        }
    }

    #[test]
    fn an_image_target_is_never_resolved_against_the_disk() {
        // The inherited `image_uri` reads the file to embed it as a data URI.
        // Whether a document may read a file is the sandbox's decision.
        let renderer = TypesetRenderer::new();
        let parser = Parser::default();

        assert_eq!(
            renderer.image_uri("../../etc/passwd", &parser, None),
            "../../etc/passwd",
            "the target is passed through untouched, not resolved and not read"
        );
    }

    /// The decoded form of a document's first paragraph.
    fn decoded(source: &str) -> DecodedInline {
        decode(&encode_document(source))
    }

    #[test]
    fn a_style_becomes_a_styled_node() {
        let text = decoded("a *b* c").text;

        assert_eq!(
            text.nodes(),
            [
                InlineNode::Text("a ".to_owned()),
                InlineNode::styled(InlineStyle::Strong, vec![InlineNode::text("b")]),
                InlineNode::Text(" c".to_owned()),
            ]
        );
    }

    #[test]
    fn nesting_survives_the_round_trip() {
        let text = decoded("*bold _and italic_*").text;

        assert_eq!(
            text.nodes(),
            [InlineNode::styled(
                InlineStyle::Strong,
                vec![
                    InlineNode::text("bold "),
                    InlineNode::styled(InlineStyle::Emphasis, vec![InlineNode::text("and italic")]),
                ]
            )]
        );
    }

    #[test]
    fn special_characters_decode_back_to_themselves() {
        assert_eq!(decoded("a < b > c & d").text.plain_text(), "a < b > c & d");
    }

    #[test]
    fn a_character_reference_resolves_as_asciidoc_says_it_should() {
        // `&lt;` in source is a character reference, not literal text, and
        // Asciidoctor renders it as `<`. The reader sees the character.
        assert_eq!(decoded("a &lt; b").text.plain_text(), "a < b");
    }

    #[test]
    fn an_escaped_ampersand_does_not_decode_twice() {
        // Here the author escaped the ampersand, so the reader must see the
        // text `&lt;`. Decoding left to right yields `&` and then the literal
        // `lt;`; successive whole-string replacements would have produced `<`.
        assert_eq!(decoded(r"a \&lt; b").text.plain_text(), "a &lt; b");
    }

    #[test]
    fn an_unsupported_construct_is_named_and_its_text_kept() {
        let decoded = decoded("See https://example.com[the site].");

        assert_eq!(decoded.unsupported, ["link"]);
        assert_eq!(decoded.text.plain_text(), "See the site.");
    }

    #[test]
    fn an_unsupported_construct_contributes_no_presentation() {
        let text = decoded("See https://example.com[the site].").text;

        assert!(
            text.nodes()
                .iter()
                .all(|node| matches!(node, InlineNode::Text(_))),
            "a construct that is not honoured must not leave a styled node behind: {:?}",
            text.nodes()
        );
    }

    #[test]
    fn a_hard_line_break_decodes_to_a_break_node() {
        let text = decoded("line one +\nline two").text;

        assert!(
            text.nodes().contains(&InlineNode::LineBreak),
            "expected a break node, got {:?}",
            text.nodes()
        );
    }

    #[test]
    fn passthrough_content_cannot_forge_a_span() {
        // The guard at the parse boundary refuses a real marker in source, so
        // the closest an author can get is the text of one. It must stay text.
        let text = decoded("pass:[a *b* c] and +++*d*+++").text;

        assert!(
            text.nodes()
                .iter()
                .all(|node| matches!(node, InlineNode::Text(_))),
            "passthrough content must not become structure: {:?}",
            text.nodes()
        );
        assert!(text.plain_text().contains("*b*"));
        assert!(text.plain_text().contains("*d*"));
    }

    #[test]
    fn a_stray_close_marker_in_the_stream_is_survivable() {
        // Not reachable from source — the guard refuses these characters — but
        // the decoder must not panic or lose the document if one appears.
        let decoded = decode(&format!("a{SPAN_CLOSE}b{SPAN_CLOSE}c"));

        assert_eq!(decoded.text.plain_text(), "abc");
    }

    #[test]
    fn an_unclosed_span_keeps_its_content() {
        let decoded = decode(&format!("a{STRONG_OPEN}b"));

        assert_eq!(
            decoded.text.plain_text(),
            "ab",
            "an unbalanced marker must not cost the reader the text inside it"
        );
    }

    mod properties {
        use proptest::prelude::*;

        use super::super::{MARKER_RANGE_END, MARKER_RANGE_START, decode};
        use super::encode_all;
        use adocpdf_core::document::InlineNode;

        /// Source text a document could plausibly contain.
        ///
        /// Noncharacters are excluded because the guard at the parse boundary
        /// refuses them outright — a document containing one never reaches the
        /// renderer, so generating them here would test the wrong claim.
        fn source_text() -> impl Strategy<Value = String> {
            let character = prop_oneof![
                // The characters that carry meaning in AsciiDoc's inline
                // grammar. Weighted heavily: they are what could become
                // structure the author did not write.
                8 => prop_oneof![
                    Just('*'), Just('_'), Just('`'), Just('^'), Just('~'), Just('#'),
                    Just('<'), Just('>'), Just('&'), Just('+'), Just('\\'), Just('{'),
                    Just('}'), Just('['), Just(']'), Just(':'), Just(';'), Just('"'),
                ],
                6 => prop::char::range('a', 'z'),
                2 => prop::char::range('0', '9'),
                2 => Just(' '),
                // No private-use characters here, and that is not an
                // oversight. `asciidoc-parser` reserves U+E000–U+E003 for its
                // own placeholders and misreads a document that contains one;
                // this generator found that, and the parse boundary now
                // refuses the range, so such a document never reaches the code
                // under test. See
                // `parser_refusal::a_parser_sentinel_in_source_is_refused`.
                1 => prop_oneof![
                    Just('\u{a9}'), Just('\u{2014}'), Just('\u{4e2d}'), Just('\u{f8ff}'),
                ],
            ];

            proptest::collection::vec(character, 1..40).prop_map(|characters| {
                let text: String = characters.into_iter().collect();
                // A leading marker-adjacent line would make the text a
                // different construct entirely; keep it to one paragraph.
                text.replace('\n', " ")
            })
        }

        /// Every character in a decoded tree's text, structure discarded.
        fn text_of(nodes: &[InlineNode]) -> String {
            let mut out = String::new();
            for node in nodes {
                node.write_plain_text(&mut out);
            }
            out
        }

        proptest! {
            /// No source text can put a structural marker into the stream the
            /// decoder reads. The markers are the alphabet structure is spelled
            /// in, so a source that could produce one could forge formatting,
            /// and — through a monospace span — a font instruction.
            #[test]
            fn no_source_text_produces_a_structural_marker(text in source_text()) {
                let encoded = encode_all(&text);

                for character in encoded.chars() {
                    prop_assert!(
                        !(MARKER_RANGE_START..=MARKER_RANGE_END).contains(&character)
                            || super::super::is_emitted_marker(character),
                        "source {text:?} produced the marker U+{:04X}",
                        character as u32
                    );
                }
            }

            /// Whatever the source said, the reader sees — no marker survives
            /// into the text, whether or not the parser made structure of it.
            #[test]
            fn no_marker_survives_into_decoded_text(text in source_text()) {
                let decoded = decode(&encode_all(&text));
                let plain = text_of(decoded.text.nodes());

                for character in plain.chars() {
                    prop_assert!(
                        !(MARKER_RANGE_START..=MARKER_RANGE_END).contains(&character),
                        "a marker reached the page from {text:?}"
                    );
                }
            }

            /// Nothing an author can type into a passthrough reaches the
            /// decoder as a structural marker.
            ///
            /// This is deliberately a claim about **markers**, not about node
            /// kinds. An earlier version asserted that everything decoded from
            /// `+++…+++` was plain text, and proptest defeated it twice:
            /// `"+++~a~"` merges its own plus signs with the delimiter, and
            /// `";; #©#"` does not end up as one passthrough either. In both
            /// cases AsciiDoc was working correctly and the test was asserting
            /// something about the wrapper rather than about safety. What
            /// matters is that the passthrough channel — the one that reaches
            /// the output with no substitution applied — cannot introduce the
            /// alphabet structure is spelled in.
            #[test]
            fn passthrough_content_never_yields_a_marker(text in source_text()) {
                let encoded = encode_all(&format!("+++{text}+++"));

                for character in encoded.chars() {
                    prop_assert!(
                        !(MARKER_RANGE_START..=MARKER_RANGE_END).contains(&character)
                            || super::super::is_emitted_marker(character),
                        "passthrough content produced the marker U+{:04X} from {text:?}",
                        character as u32
                    );
                }

                let decoded = decode(&encoded);
                let mut plain = String::new();
                for node in decoded.text.nodes() {
                    node.write_plain_text(&mut plain);
                }
                for character in plain.chars() {
                    prop_assert!(
                        !(MARKER_RANGE_START..=MARKER_RANGE_END).contains(&character),
                        "a marker reached the page from passthrough content {text:?}"
                    );
                }
            }
        }
    }
}
