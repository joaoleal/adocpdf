//! Turning AsciiDoc source into the document model.
//!
//! The upstream parser is wrapped rather than used directly by the application,
//! so swapping it would touch this file and nothing else.
//!
//! Two behaviours are worth knowing about:
//!
//! - The parser runs in the most restrictive safe mode, so a document cannot
//!   reach outside itself through includes or file-reading macros. That is the
//!   `project-sandbox` requirement enforced at the point where document content
//!   could otherwise widen file access.
//! - Inline content comes from the parser's rendered output, with this crate's
//!   own [`crate::inline::TypesetRenderer`] installed in place
//!   of the built-in HTML one. Taking the **source span** instead — as this
//!   adapter once did — leaves inline formatting uninterpreted and puts `*` and
//!   `_` on the page as literal characters. Taking the rendered output *without*
//!   replacing the renderer would put HTML tags there instead.

use std::panic::{AssertUnwindSafe, catch_unwind};

use adocpdf_adapters::calendar::unix_timestamp;
use adocpdf_core::document::{
    Admonition, AdmonitionKind, Block, BreakKind, Container, ContainerKind, Document, HeadingLevel,
    InlineText, List, ListItem, ListKind, Paragraph, Quotation, QuotationKind, Section, Verbatim,
    VerbatimKind,
};
use adocpdf_core::presentation::{
    Alignment, LEAD_ROLE, ListForm, ListMarker, ListPresentation, ListStart, ParagraphPresentation,
};
use adocpdf_core::theme::ThemeId;
use adocpdf_domain::error::{DomainError, SourceLocation};
use adocpdf_domain::ports::{Date, DocumentParser, ParseOutcome, SkippedConstruct};
use asciidoc_parser::blocks::FindBlocks;
use asciidoc_parser::blocks::{
    AdmonitionVariant, Block as SourceBlock, BreakType, CompoundDelimitedContext, IsBlock,
    ListItemMarker, ListType as SourceListType, QuoteType as SourceQuoteType, SimpleBlockStyle,
};
use asciidoc_parser::parser::ModificationContext;
use asciidoc_parser::warnings::WarningType;
use asciidoc_parser::{Document as ParsedDocument, HasSpan, Parser, ReferenceTime, SafeMode, Span};

use crate::inline::{
    DecodedInline, TypesetRenderer, decode, decode_keeping_line_breaks, decode_preserving_text,
    is_parser_sentinel, is_reserved_marker,
};

/// The attribute an ordered list uses to declare the number it counts from.
///
/// For example:
///
/// ```asciidoc
/// [start=4]
/// . the fourth thing
/// ```
pub const START_ATTRIBUTE: &str = "start";

/// The block attribute a section uses to declare its theme.
///
/// For example:
///
/// ```asciidoc
/// [theme=wide]
/// == Appendix
/// ```
pub const THEME_ATTRIBUTE: &str = "theme";

/// Parses AsciiDoc with the upstream parser.
#[derive(Debug, Clone, Copy, Default)]
pub struct AsciidocParser;

impl AsciidocParser {
    /// Creates a parser.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl DocumentParser for AsciidocParser {
    fn parse(&self, source: &str, origin: &str, today: Date) -> Result<ParseOutcome, DomainError> {
        refuse_input_that_would_not_terminate(source, origin)?;
        refuse_input_with_an_unterminating_carriage_return(source, origin)?;
        refuse_input_that_could_forge_structure(source, origin)?;

        let mut parser = Parser::default()
            // The most restrictive mode: no includes, no file-reading macros.
            // A document must not be able to widen its own access.
            .with_safe_mode(SafeMode::Secure)
            // The reference time is injected so that a document resolving a
            // date attribute renders the same whenever it is built.
            .with_reference_time(ReferenceTime::from_unix_timestamp(unix_timestamp(today)))
            // Inline substitutions are rendered as structure this crate can
            // decode, rather than as the HTML the built-in renderer produces.
            .with_inline_substitution_renderer(TypesetRenderer::new())
            // Asks the parser to say when a document references an attribute
            // that was never set. Without this it silently leaves the
            // reference as written, and the author is never told which name
            // went unresolved. The document may still override it.
            .with_intrinsic_attribute(
                "attribute-missing",
                "warn",
                ModificationContext::ApiOrDocumentBody,
            );

        let parsed = parse_without_letting_a_panic_escape(&mut parser, source, origin)?;

        let mut mapper = Mapper {
            skipped: Vec::new(),
        };

        let mut document = Document::new();
        if let Some(title) = parsed.doctitle() {
            // The document title is inline content like any other. It used to
            // be taken verbatim from `doctitle()`, which returns *rendered*
            // output — so a title reading `= *Bold* Title` put a literal
            // `<strong>` on the page.
            document = document.with_title(mapper.inline(title, SourceLocation::START));
        }
        for block in parsed.child_blocks() {
            for mapped in mapper.blocks(block) {
                document = document.with_block(mapped);
            }
        }

        // An unresolved attribute reference is not a block the mapper walks
        // past — the parser reports it as a warning, with the name and the
        // place it was referenced. Both are better than anything reconstructed
        // from the rendered text afterwards.
        for warning in parsed.warnings() {
            if let WarningType::SkippingReferenceToMissingAttribute(name) = &warning.warning {
                mapper.skipped.push(SkippedConstruct {
                    construct: format!("reference to attribute {name:?}, which is not set"),
                    location: location_of(warning.source),
                });
            }
        }

        Ok(ParseOutcome {
            document,
            skipped: mapper.skipped,
        })
    }
}

/// Characters `asciidoc-parser` cannot be trusted to finish reading.
///
/// # The defect
///
/// `asciidoc-parser` 0.29.19 does not terminate on a range of documents
/// containing a vertical tab or a form feed. `Parser::parse("\u{c}")` spins
/// indefinitely — verified against the crate directly, with none of this
/// module's code involved, so this is upstream and not a mapping error here.
///
/// # Why the whole character is refused, rather than a shape it appears in
///
/// Because three narrower rules were tried and all three were wrong. The fuzz
/// target found each of them in turn:
///
/// 1. First the guard refused these characters in a **whitespace-only
///    document**, which is the shape every example then in hand had. Fuzzing
///    later produced `"[;;\u{a}\u{b}"` — a document with real content — which
///    hangs just the same.
/// 2. Then it refused them on a **line holding nothing else**, which covered
///    every case measured at that point. Fuzzing then produced
///    `";toc::  \u{c}"`, where the character shares its line with content.
/// 3. What survives is the character itself.
///
/// Each narrower rule was an attempt to model an upstream defect precisely
/// enough to refuse only what genuinely hangs. That is the wrong goal: the
/// defect is not a specification, it is a bug, and its shape can change with
/// any input nobody has tried yet. A rule about the character cannot be
/// outflanked that way.
///
/// # What this costs
///
/// `"Hello\u{c}world"` used to render and is now refused. That was a
/// deliberate allowance, and giving it up is the price of a rule that cannot
/// be outflanked. It is a small price: a vertical tab or form feed in AsciiDoc
/// source is malformed text in any case, produced by no editor and meaning
/// nothing in the language. The specification permits a conservative refusal
/// whose cost is bounded and stated, and this one is bounded to two characters
/// that no document should contain.
///
/// # When to delete this
///
/// When a released `asciidoc-parser` terminates on these inputs and this
/// workspace has adopted it. The regression tests in
/// `tests/hang_regressions.rs` do not depend on this function and will then
/// pass because the defect is gone rather than because it is unreachable.
const CHARACTERS_UPSTREAM_CANNOT_PARSE: [char; 2] = ['\u{b}', '\u{c}'];

/// Declines a document that the parser would not return from.
///
/// # Errors
///
/// Returns [`DomainError::ParseFailed`] naming the offending character when
/// `source` contains any of [`CHARACTERS_UPSTREAM_CANNOT_PARSE`].
fn refuse_input_that_would_not_terminate(source: &str, origin: &str) -> Result<(), DomainError> {
    let Some((offset, offender)) = source
        .char_indices()
        .find(|(_, character)| CHARACTERS_UPSTREAM_CANNOT_PARSE.contains(character))
    else {
        return Ok(());
    };

    Err(DomainError::ParseFailed {
        path: origin.to_owned(),
        location: location_of_offset(source, offset),
        reason: format!(
            "the document contains U+{:04X}; asciidoc-parser 0.29.19 does not reliably \
             terminate on a document containing that character, so it is refused rather \
             than rendered",
            offender as u32
        ),
    })
}

/// Declines a document holding a carriage return the parser cannot finish
/// reading.
///
/// # The defect
///
/// `asciidoc-parser` 0.29.19 does not terminate on a document containing a
/// carriage return immediately followed by whitespace that is not a line feed.
/// `Parser::parse("\r\r")` spins indefinitely, and so do `"\r\t"`, `"\r "` and
/// `"= T\n\n\r\r"` — the last of which has a title and real content in it. It
/// was found by the `parse_plan_emit` fuzz target, which minimised it to
/// `"\n\r\r\r"`.
///
/// This is a second defect of the same family as
/// [`CHARACTERS_UPSTREAM_CANNOT_PARSE`], not the same one. That guard refuses
/// only documents with no real content, which is the whole shape of the
/// vertical-tab and form-feed hang. This one hangs on documents that do have
/// content, so it needs a condition of its own.
///
/// # What was measured
///
/// Every string of length four or less over `{CR, LF, tab, space, 'a'}` was
/// probed against the real parser — 780 inputs, of which 138 do not return.
/// **Every one of those 138 contains this pattern**, and so does every longer
/// hanging case found by hand. The pattern is therefore known to be necessary;
/// no input is missed.
///
/// # Why the refusal is deliberately wider than the defect
///
/// The pattern is necessary but not sufficient: 81 of the probed inputs
/// contain it and still return — `"\r\ra"` and `"a\r\r"` among them. Refusing
/// those is a choice, made because the exact condition depends on block
/// structure in a way that is not stable enough to encode against an upstream
/// defect that should be fixed rather than modelled. The specification permits
/// it: a refusal may be conservative where the alternative is a render that
/// never finishes, and a renderer that declines a stray carriage return is a
/// nuisance where one that never returns is a vulnerability.
///
/// The cost is bounded, and deliberately so:
///
/// - **Windows line endings are unaffected.** In CRLF text every carriage
///   return is followed by a line feed, which this pattern excludes. A CRLF
///   document of any size never trips it.
/// - **A carriage return followed by text is unaffected**, so `"\ra"` still
///   renders.
///
/// What is refused is a bare carriage return followed by another space, tab or
/// carriage return — which no line-ending convention produces, and which is
/// malformed text by any reading.
///
/// # When to delete this
///
/// When a released `asciidoc-parser` terminates on these inputs and this
/// workspace has adopted it. The regression tests do not depend on this
/// function and will then pass because the defect is gone.
fn refuse_input_with_an_unterminating_carriage_return(
    source: &str,
    origin: &str,
) -> Result<(), DomainError> {
    let mut characters = source.char_indices().peekable();

    while let Some((offset, character)) = characters.next() {
        if character != '\r' {
            continue;
        }

        let Some(&(_, next)) = characters.peek() else {
            // A carriage return ending the document is fine: there is nothing
            // after it for the parser to get stuck on.
            break;
        };

        if next.is_whitespace() && next != '\n' {
            return Err(DomainError::ParseFailed {
                path: origin.to_owned(),
                location: location_of_offset(source, offset),
                reason: format!(
                    "a carriage return is followed by U+{:04X} rather than by a line feed; \
                     asciidoc-parser 0.29.19 does not terminate on that sequence, so it is \
                     refused rather than rendered",
                    next as u32
                ),
            });
        }
    }

    Ok(())
}

// Why two ranges are refused, and why the predicates live in `crate::inline`
// rather than here.
//
// **This crate's markers.** Unicode permanently reserves U+FDD0–U+FDEF as
// *noncharacters*: they are guaranteed never to be assigned, and the standard
// forbids their use in open interchange. That is what makes them usable as a
// marker alphabet — a legitimate AsciiDoc document does not contain one, so a
// marker in the parser's output can only have come from this crate. The
// guarantee only holds if it is enforced: `pass:[…]` and `+++…+++` reach the
// rendered output verbatim, so an author could otherwise type a marker
// straight into the stream the decoder reads and forge inline structure.
// Refusing the range at the door closes that channel, and is why the injection
// property can be stated as a property rather than as a list of examples.
//
// **The parser's sentinels.** U+E000 and U+E001 bracket a cross-reference
// placeholder in the parser's rendered output, and U+E002 and U+E003 bracket a
// footnote marker (`src/content/content.rs:138`–`154`). The crate's own comment
// says they "cannot collide with user text", and that is not so: a document can
// type one. On `"*\u{e000}<<>>"` the parser reaches
// `debug_assert!(false, "xref placeholder index {body:?} out of range")` — a
// panic in a debug build, a corrupt placeholder in a release one. It is the
// same hazard this crate avoids by drawing its markers from noncharacters
// rather than from the private use area, which documents legitimately contain.
//
// **Why `crate::inline` owns both predicates.** This guard is not the only
// place a reserved character can appear. A numeric character reference is
// resolved *after* the guard has run, by the decoder, which has to refuse the
// same two ranges. When each side kept its own copy they drifted: the decoder
// knew about the noncharacters and not about the sentinels, so `&#xE000;` went
// straight past a guard written to refuse U+E000. One definition, used by both.

/// Declines a document containing a character reserved as a marker, by this
/// crate or by the parser it wraps.
///
/// # Errors
///
/// Returns [`DomainError::ParseFailed`] naming the offending code point when
/// `source` contains a character this crate reserves for markers or one the
/// parser reserves for its own placeholders.
fn refuse_input_that_could_forge_structure(source: &str, origin: &str) -> Result<(), DomainError> {
    let Some((offset, offender)) = source
        .char_indices()
        .find(|(_, character)| is_reserved_marker(*character) || is_parser_sentinel(*character))
    else {
        return Ok(());
    };

    let reason = if is_reserved_marker(offender) {
        format!(
            "the document contains U+{:04X}, a Unicode noncharacter reserved by this \
             renderer for marking inline structure; such a character must not appear in \
             interchanged text, and accepting it would let the document forge formatting \
             it did not write",
            offender as u32
        )
    } else {
        format!(
            "the document contains U+{:04X}, which asciidoc-parser 0.29.19 reserves for \
             its own cross-reference and footnote placeholders; a document containing one \
             makes the parser misread its own output, so it is refused rather than \
             rendered",
            offender as u32
        )
    };

    Err(DomainError::ParseFailed {
        path: origin.to_owned(),
        location: location_of_offset(source, offset),
        reason,
    })
}

/// Runs the upstream parser, turning a panic inside it into a refusal.
///
/// The three guards above refuse inputs the parser cannot *finish*. This one
/// covers the other way it fails: `asciidoc-parser` 0.29.19 panics on an inline
/// `image:` or `icon:` macro with no target, because the regex makes the target
/// group optional and the replacer indexes it unconditionally
/// (`content/macros.rs:291`). `image:[alt]` is enough — a typo in an ordinary
/// document, not an adversarial input. A second case, found by the fuzz run
/// that verified this guard, trips a `debug_assert!` on a block attribute list
/// combining `%` shorthands (`attributes/element_attribute.rs:509`); it is
/// contained here too, without a rule of its own.
///
/// It is caught here rather than refused before parsing because no rule about
/// the source text can decide it. Attribute references are substituted *before*
/// macros, so the macro name need never appear in the source:
///
/// ```asciidoc
/// :foo: imag
/// {foo}e:[]
/// ```
///
/// still panics, and any guard reading the source misses it. Catching the
/// unwind is the only rule that cannot be outflanked, and it covers whatever
/// upstream panics next as well.
///
/// The cost, stated: this crate's own [`TypesetRenderer`] runs *inside* `parse`
/// as a substitution callback, so a panic in it would be reported as a refusal
/// rather than crashing. The process-wide panic hook is deliberately left
/// alone — a library must not silence panic reporting for its host — so the
/// hook's message still reaches stderr ahead of the error returned here.
///
/// # Errors
///
/// Returns [`DomainError::ParseFailed`] when the parser panics.
fn parse_without_letting_a_panic_escape(
    parser: &mut Parser,
    source: &str,
    origin: &str,
) -> Result<ParsedDocument<'static>, DomainError> {
    // `AssertUnwindSafe` because the parser is dropped on the error path below
    // and never read after a panic; nothing observes whatever state it was left
    // in.
    catch_unwind(AssertUnwindSafe(|| parser.parse(source))).map_err(|_| DomainError::ParseFailed {
        path: origin.to_owned(),
        location: SourceLocation::START,
        reason: "asciidoc-parser 0.29.19 panicked while parsing this document. Two \
                 causes are known: an inline image: or icon: macro written with no \
                 target, as in image:[alt], and a block attribute list combining % \
                 shorthands. The document is refused rather than rendered, because a \
                 panic cannot be recovered from far enough to trust what was parsed"
            .to_owned(),
    })
}

/// Locates a byte offset within the source, counting from one.
fn location_of_offset(source: &str, offset: usize) -> SourceLocation {
    let consumed = &source[..offset];
    let line = consumed.matches('\n').count() + 1;
    let column = consumed.rfind('\n').map_or_else(
        || consumed.chars().count(),
        |index| consumed[index + 1..].chars().count(),
    ) + 1;

    SourceLocation::new(
        u32::try_from(line).unwrap_or(1),
        u32::try_from(column).unwrap_or(1),
    )
}

/// Maps parsed blocks into the document model, collecting what it cannot.
struct Mapper {
    skipped: Vec<SkippedConstruct>,
}

impl Mapper {
    /// Maps one parsed block into zero or more model blocks.
    ///
    /// Zero or more rather than exactly one because some parsed blocks are not
    /// content in their own right: a preamble is a transparent container around
    /// the blocks before the first section, and a document attribute is a
    /// directive the parser has already applied. Flattening the first and
    /// dropping the second keeps them out of the skipped list, where they would
    /// wrongly suggest something was lost.
    fn blocks(&mut self, block: &SourceBlock<'_>) -> Vec<Block> {
        match block {
            SourceBlock::Simple(simple) => self.simple(simple, block),

            // A discrete heading arrives as a section that holds nothing: the
            // parser gives it its own context and leaves the blocks after it
            // where they were. Mapping it to a section all the same would put
            // a container around nothing and give the engine an outline entry
            // for a heading that asked to stay out of the structure.
            SourceBlock::Section(source_section)
                if source_section.resolved_context().as_ref() == FLOATING_TITLE_CONTEXT =>
            {
                let text = self.inline(
                    source_section.section_title(),
                    location_of(source_section.span()),
                );
                vec![Block::Heading {
                    text,
                    level: clamp_level(source_section.level()),
                }]
            }

            SourceBlock::Section(source_section) => self.section(source_section),

            SourceBlock::RawDelimited(raw) => {
                match Self::verbatim(raw) {
                    // A comment maps to nothing, and nothing cannot carry a title.
                    mapped if mapped.is_empty() => mapped,
                    mut mapped => self.titled(block, mapped.remove(0)),
                }
            }

            SourceBlock::Admonition(admonition) => self.admonition(admonition, block),

            SourceBlock::Quote(quote) => self.quotation(quote, block),

            SourceBlock::CompoundDelimited(compound) => {
                let kind = match compound.context_kind() {
                    CompoundDelimitedContext::Example => ContainerKind::Example,
                    CompoundDelimitedContext::Sidebar => ContainerKind::Sidebar,
                    CompoundDelimitedContext::Open => ContainerKind::Open,
                };
                let body = self.children(compound.child_blocks());

                self.titled(block, Block::Container(Container::new(kind, body)))
            }

            SourceBlock::Break(source_break) => {
                let kind = match source_break.type_() {
                    BreakType::Page => BreakKind::Page,
                    BreakType::Thematic => BreakKind::Thematic,
                };
                vec![Block::Break(kind)]
            }

            SourceBlock::List(list) => self.list(list, block),

            // A transparent container: its children belong to whatever holds it.
            SourceBlock::Preamble(preamble) => preamble
                .child_blocks()
                .flat_map(|child| self.blocks(child))
                .collect(),

            // Already applied by the parser. Nothing is lost by not rendering it.
            SourceBlock::DocumentAttribute(_) => Vec::new(),

            other => {
                self.skip(describe(other), other.span());
                Vec::new()
            }
        }
    }

    /// Maps a simple block: a paragraph, or an indented literal one.
    fn simple(
        &mut self,
        simple: &asciidoc_parser::blocks::SimpleBlock<'_>,
        block: &SourceBlock<'_>,
    ) -> Vec<Block> {
        // An indented paragraph is a literal block, not body text: the
        // indentation is the author asking for it to be left alone.
        if simple.style() != SimpleBlockStyle::Paragraph {
            let content = Self::verbatim_content(simple.content());
            let kind = match simple.style() {
                SimpleBlockStyle::Listing => VerbatimKind::Listing,
                SimpleBlockStyle::Source => VerbatimKind::Source,
                _ => VerbatimKind::Literal,
            };
            return self.titled(block, Block::Verbatim(Verbatim::new(kind, content)));
        }

        let rendered = simple.content().rendered().trim_end();
        let text = self.inline(rendered, location_of(simple.span()));
        if text.is_empty() {
            return Vec::new();
        }

        let presentation = self.paragraph_presentation(block);
        self.titled(
            block,
            Block::Paragraph(Paragraph::new(text).with_presentation(presentation)),
        )
    }

    /// Reads a paragraph's attribute list for how it asked to be set.
    ///
    /// A role this renderer has no meaning for is reported and its paragraph
    /// still rendered, which is the same promise the inline path makes: the
    /// omission is never silent and no content is lost to it.
    fn paragraph_presentation(&mut self, block: &SourceBlock<'_>) -> ParagraphPresentation {
        let mut presentation = ParagraphPresentation::body();

        for role in block.roles() {
            if role == LEAD_ROLE {
                presentation = presentation.as_lead();
            } else if let Some(alignment) = Alignment::from_role(role) {
                presentation = presentation.with_alignment(alignment);
            } else {
                self.skip(format!("role {role:?}"), block.span());
            }
        }

        presentation
    }

    /// Maps a section and everything nested inside it.
    fn section(
        &mut self,
        source_section: &asciidoc_parser::blocks::SectionBlock<'_>,
    ) -> Vec<Block> {
        let heading = self.inline(
            source_section.section_title(),
            location_of(source_section.span()),
        );
        let mut section = Section::new(heading, clamp_level(source_section.level()));

        if let Some(id) = self.declared_theme(source_section) {
            section = section.with_theme(id);
        }

        for child in source_section.child_blocks() {
            for mapped in self.blocks(child) {
                section = section.with_block(mapped);
            }
        }

        vec![Block::Section(section)]
    }

    /// Maps an admonition, in either of the two forms it can take.
    fn admonition(
        &mut self,
        admonition: &asciidoc_parser::blocks::AdmonitionBlock<'_>,
        block: &SourceBlock<'_>,
    ) -> Vec<Block> {
        {
            {
                let kind = match admonition.variant() {
                    AdmonitionVariant::Note => AdmonitionKind::Note,
                    AdmonitionVariant::Tip => AdmonitionKind::Tip,
                    AdmonitionVariant::Important => AdmonitionKind::Important,
                    AdmonitionVariant::Caution => AdmonitionKind::Caution,
                    AdmonitionVariant::Warning => AdmonitionKind::Warning,
                };

                // The single-paragraph form carries its text directly; the
                // delimited form carries child blocks instead.
                let body = if let Some(content) = admonition.content() {
                    vec![Block::Paragraph(Paragraph::new(self.inline(
                        content.rendered().trim_end(),
                        location_of(admonition.span()),
                    )))]
                } else {
                    self.children(admonition.child_blocks())
                };

                self.titled(block, Block::Admonition(Admonition::new(kind, body)))
            }
        }
    }

    /// Maps a quotation, prose or verse, with whatever attribution it carries.
    fn quotation(
        &mut self,
        quote: &asciidoc_parser::blocks::QuoteBlock<'_>,
        block: &SourceBlock<'_>,
    ) -> Vec<Block> {
        {
            {
                let kind = match quote.type_() {
                    SourceQuoteType::Verse => QuotationKind::Verse,
                    SourceQuoteType::Quote => QuotationKind::Quote,
                };

                let body = if let Some(content) = quote.content() {
                    let rendered = content.rendered();
                    let rendered = rendered.trim_end();
                    let location = location_of(quote.span());
                    // A verse's line endings are the content — the shape of the
                    // lines is what makes it a verse — so they become explicit
                    // breaks. A quote is prose and is filled like any other
                    // paragraph.
                    let text = if kind == QuotationKind::Verse {
                        self.inline_keeping_line_breaks(rendered, location)
                    } else {
                        self.inline(rendered, location)
                    };
                    vec![Block::Paragraph(Paragraph::new(text))]
                } else {
                    self.children(quote.child_blocks())
                };

                let mut quotation = Quotation::new(kind, body);
                let location = location_of(quote.span());
                if let Some(attribution) = quote.attribution() {
                    quotation = quotation.with_attribution(self.inline(attribution, location));
                }
                if let Some(citation) = quote.citetitle() {
                    quotation = quotation.with_citation(self.inline(citation, location));
                }

                self.titled(block, Block::Quotation(quotation))
            }
        }
    }

    /// Reads a section's declared theme, recording a malformed one as skipped.
    ///
    /// A theme name that is not a well-formed identifier is reported rather than
    /// passed along: an unusable name would otherwise surface later as a
    /// confusing "unknown theme" against a name the author never typed.
    fn declared_theme(
        &mut self,
        section: &asciidoc_parser::blocks::SectionBlock<'_>,
    ) -> Option<ThemeId> {
        let attribute = section.attrlist()?.named_attribute(THEME_ATTRIBUTE)?;
        let value = attribute.value();

        match ThemeId::new(value) {
            Ok(id) => Some(id),
            Err(error) => {
                self.skip(
                    format!("theme attribute {value:?} ({error})"),
                    section.span(),
                );
                None
            }
        }
    }

    /// Maps a delimited block whose content is kept exactly as written.
    ///
    /// A comment block is dropped and *not* reported: the author asked for it
    /// to be left out, so naming it as a skipped construct would report the
    /// renderer doing exactly what was asked.
    fn verbatim(raw: &asciidoc_parser::blocks::RawDelimitedBlock<'_>) -> Vec<Block> {
        let context = raw.resolved_context().to_string();
        if context == "comment" {
            return Vec::new();
        }

        let kind = match context.as_str() {
            "literal" => VerbatimKind::Literal,
            "source" => VerbatimKind::Source,
            // A passthrough or stem block has no target format to pass through
            // to, so its content is set as written, which is what a literal
            // block is.
            _ => VerbatimKind::Listing,
        };

        vec![Block::Verbatim(Verbatim::new(
            kind,
            Self::verbatim_content(raw.content()),
        ))]
    }

    /// The text of a verbatim block, with the parser's escaping undone.
    ///
    /// Verbatim content still passes through the special-character
    /// substitution, so `<` arrives as `&lt;`. Decoding gives back what the
    /// author typed; there is no structure to find, because no other
    /// substitution ran.
    fn verbatim_content(content: &asciidoc_parser::content::Content<'_>) -> String {
        // Decoded, but otherwise untouched: a verbatim block is read for its
        // characters, so every space and newline in it is one the author typed.
        // Paragraphs are filled instead — see `inline`.
        decode_preserving_text(content.rendered().trim_end())
            .text
            .plain_text()
    }

    /// Maps a list and its items.
    fn list(
        &mut self,
        list: &asciidoc_parser::blocks::ListBlock<'_>,
        block: &SourceBlock<'_>,
    ) -> Vec<Block> {
        let kind = match list.type_() {
            SourceListType::Ordered => ListKind::Ordered,
            SourceListType::Description => ListKind::Description,
            // A callout list belongs with the callouts it explains, which are
            // tier 5. Setting it as an ordinary list keeps its text.
            _ => ListKind::Unordered,
        };

        let mut items = Vec::new();
        for child in list.child_blocks() {
            let SourceBlock::ListItem(source_item) = child else {
                continue;
            };

            let mut item = ListItem::new(self.children(source_item.child_blocks()));
            if let ListItemMarker::DefinedTerm { term, source, .. } = source_item.list_item_marker()
            {
                item = item.with_term(self.inline(term.rendered(), location_of(source)));
            }
            // The parser has already taken the `[x]` out of the item's text, so
            // nothing has to strip it here — and nothing can put it back.
            if let Some(checked) = source_item.checkbox() {
                item = item.with_checkbox(checked);
            }
            items.push(item);
        }

        let presentation = self.list_presentation(block, list.is_checklist());

        self.titled(
            block,
            Block::List(List::new(kind, items).with_presentation(presentation)),
        )
    }

    /// Reads a list's attribute list for how it asked to be set.
    ///
    /// An attribute this renderer does not honour is reported by name, and the
    /// list still renders with its default presentation: a presentation choice
    /// must never cost an author their content.
    fn list_presentation(
        &mut self,
        block: &SourceBlock<'_>,
        is_checklist: bool,
    ) -> ListPresentation {
        let mut presentation = ListPresentation::stacked();

        // Read from the items rather than declared, so it comes first and a
        // declared style can still have its say.
        if is_checklist {
            presentation = presentation.with_form(ListForm::Checklist);
        }

        if let Some(attrlist) = block.attrlist() {
            if let Some(style) = attrlist.block_style() {
                if let Some(marker) = ListMarker::from_style(style) {
                    presentation = presentation.with_marker(marker);
                } else if let Some(form) = ListForm::from_style(style) {
                    presentation = presentation.with_form(form);
                } else {
                    self.skip(format!("list style {style:?}"), block.span());
                }
            }

            if let Some(start) = attrlist.named_attribute(START_ATTRIBUTE) {
                match ListStart::parse(start.value()) {
                    Ok(declared) => presentation = presentation.with_start(declared),
                    Err(error) => self.skip(error.to_string(), block.span()),
                }
            }
        }

        for role in block.roles() {
            self.skip(format!("role {role:?}"), block.span());
        }

        presentation
    }

    /// Maps a block's children.
    fn children<'a>(&mut self, blocks: impl Iterator<Item = &'a SourceBlock<'a>>) -> Vec<Block> {
        blocks.flat_map(|child| self.blocks(child)).collect()
    }

    /// Wraps a mapped block in its title, if the source gave it one.
    fn titled(&mut self, source: &SourceBlock<'_>, mapped: Block) -> Vec<Block> {
        match source.title() {
            Some(title) => {
                let title = self.inline(title, location_of(source.span()));
                vec![Block::Titled {
                    title,
                    block: Box::new(mapped),
                }]
            }
            None => vec![mapped],
        }
    }

    /// Decodes a run of rendered inline content, recording what was skipped.
    ///
    /// The location is the enclosing block's. The upstream renderer is handed
    /// no span for an inline construct — none of its parameter structs carries
    /// one — so the block is the finest granularity available.
    fn inline(&mut self, rendered: &str, location: SourceLocation) -> InlineText {
        self.inline_decoded(decode(rendered), location)
    }

    /// The same, for a block whose line endings the author chose.
    ///
    /// Only a verse asks for this. Everything else is prose, and prose is
    /// filled.
    fn inline_keeping_line_breaks(
        &mut self,
        rendered: &str,
        location: SourceLocation,
    ) -> InlineText {
        self.inline_decoded(decode_keeping_line_breaks(rendered), location)
    }

    fn inline_decoded(&mut self, decoded: DecodedInline, location: SourceLocation) -> InlineText {
        for construct in decoded.unsupported {
            self.skipped.push(SkippedConstruct {
                construct,
                location,
            });
        }

        decoded.text
    }

    fn skip(&mut self, construct: String, span: Span<'_>) {
        self.skipped.push(SkippedConstruct {
            construct,
            location: location_of(span),
        });
    }
}

/// The context the parser gives a heading that opens no section.
const FLOATING_TITLE_CONTEXT: &str = "floating_title";

/// Names a block the way an author would recognise it.
fn describe(block: &SourceBlock<'_>) -> String {
    match block {
        SourceBlock::Simple(_) => "paragraph",
        SourceBlock::Media(_) => "image, video or audio block",
        SourceBlock::Section(_) => "section",
        SourceBlock::List(_) => "list",
        SourceBlock::ListItem(_) => "list item",
        SourceBlock::RawDelimited(_) => "literal, listing or passthrough block",
        SourceBlock::CompoundDelimited(_) => "example, sidebar or open block",
        SourceBlock::Break(_) => "thematic or page break",
        SourceBlock::Admonition(_) => "admonition",
        SourceBlock::Quote(_) => "quote block",
        SourceBlock::Table(_) => "table",
        SourceBlock::Toc(_) => "table of contents",
        SourceBlock::Preamble(_) => "preamble",
        SourceBlock::DocumentAttribute(_) => "document attribute",
        _ => "block",
    }
    .to_owned()
}

/// Converts a source span into a location.
fn location_of(span: Span<'_>) -> SourceLocation {
    SourceLocation::new(
        u32::try_from(span.line()).unwrap_or(1),
        u32::try_from(span.col()).unwrap_or(1),
    )
}

/// Brings a heading level into the range the model allows.
///
/// The parser reports the level the source asked for; the model permits 1 to 6.
/// Clamping keeps a too-deep heading visible as a heading rather than dropping
/// it, which is the lesser distortion.
#[allow(
    clippy::expect_used,
    reason = "the value is clamped into the accepted range on the line above, so \
              construction cannot fail"
)]
fn clamp_level(level: usize) -> HeadingLevel {
    let clamped = level.clamp(
        usize::from(HeadingLevel::MIN),
        usize::from(HeadingLevel::MAX),
    );

    HeadingLevel::new(u8::try_from(clamped).unwrap_or(HeadingLevel::MIN))
        .expect("a clamped level is within the accepted range")
}
