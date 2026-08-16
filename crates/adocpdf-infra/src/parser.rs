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
//! - Paragraph text is taken from the **source span**, not from the parser's
//!   rendered output. Rendered output is HTML, and this renderer does not
//!   produce HTML — feeding it through would put literal tags on the page.
//!   Taking the source means inline formatting is not interpreted yet, which
//!   matches the change's stated non-goal.

use adocpdf_core::document::{Block, Document, HeadingLevel, InlineText, Paragraph, Section};
use adocpdf_core::theme::ThemeId;
use adocpdf_domain::error::{DomainError, SourceLocation};
use adocpdf_domain::ports::{Date, DocumentParser, ParseOutcome, SkippedConstruct};
use asciidoc_parser::blocks::FindBlocks;
use asciidoc_parser::blocks::{Block as SourceBlock, IsBlock};
use asciidoc_parser::{HasSpan, Parser, ReferenceTime, SafeMode, Span};

use crate::clock::unix_timestamp;

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
    fn parse(&self, source: &str, _origin: &str, today: Date) -> Result<ParseOutcome, DomainError> {
        let parsed = Parser::default()
            // The most restrictive mode: no includes, no file-reading macros.
            // A document must not be able to widen its own access.
            .with_safe_mode(SafeMode::Secure)
            // The reference time is injected so that a document resolving a
            // date attribute renders the same whenever it is built.
            .with_reference_time(ReferenceTime::from_unix_timestamp(unix_timestamp(today)))
            .parse(source);

        let mut mapper = Mapper {
            skipped: Vec::new(),
        };

        let mut document = Document::new();
        if let Some(title) = parsed.doctitle() {
            document = document.with_title(InlineText::new(title));
        }
        for block in parsed.child_blocks() {
            for mapped in mapper.blocks(block) {
                document = document.with_block(mapped);
            }
        }

        Ok(ParseOutcome {
            document,
            skipped: mapper.skipped,
        })
    }
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
            SourceBlock::Simple(simple) => {
                let text = simple.content().original().data().trim_end();
                if text.is_empty() {
                    Vec::new()
                } else {
                    vec![Block::Paragraph(Paragraph::new(InlineText::new(text)))]
                }
            }

            SourceBlock::Section(source_section) => {
                let mut section = Section::new(
                    InlineText::new(source_section.section_title()),
                    clamp_level(source_section.level()),
                );

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

    fn skip(&mut self, construct: String, span: Span<'_>) {
        self.skipped.push(SkippedConstruct {
            construct,
            location: location_of(span),
        });
    }
}

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
