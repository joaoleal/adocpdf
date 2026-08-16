//! The document model: the constructs this renderer understands.
//!
//! Deliberately small. It covers the document title, nested sections,
//! paragraphs and inline text — the set the current specification names as
//! supported — and nothing else. Constructs outside it are reported as skipped
//! by the parser adapter rather than represented here as a half-measure.

use std::error::Error;
use std::fmt;

use crate::theme::ThemeId;

/// A run of text with no internal structure.
///
/// Text is stored exactly as it appeared in the source. It is *content*, so it
/// is never restricted to an alphabet the way an identifier or a family name
/// is; instead it passes through the escaper on its way to the output. Storing
/// it verbatim is what lets that escaping be the single chokepoint.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InlineText(String);

impl InlineText {
    /// Creates inline text from source text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The text, exactly as it appeared in the source.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether there is no text at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for InlineText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// How deeply a heading is nested.
///
/// Levels run from 1 (the outermost section under the document title) to
/// [`HeadingLevel::MAX`], matching the depth AsciiDoc itself allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeadingLevel(u8);

impl HeadingLevel {
    /// The outermost heading level.
    pub const MIN: u8 = 1;
    /// The innermost heading level.
    pub const MAX: u8 = 6;

    /// Creates a heading level.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidHeadingLevel`] when the level is outside
    /// [`HeadingLevel::MIN`]..=[`HeadingLevel::MAX`].
    pub fn new(level: u8) -> Result<Self, InvalidHeadingLevel> {
        if (Self::MIN..=Self::MAX).contains(&level) {
            Ok(Self(level))
        } else {
            Err(InvalidHeadingLevel { level })
        }
    }

    /// The level as a number.
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }
}

/// A heading level outside the permitted range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidHeadingLevel {
    /// The level that was proposed.
    pub level: u8,
}

impl fmt::Display for InvalidHeadingLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "heading level must be between {} and {}, got {}",
            HeadingLevel::MIN,
            HeadingLevel::MAX,
            self.level
        )
    }
}

impl Error for InvalidHeadingLevel {}

/// A paragraph of body text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    text: InlineText,
}

impl Paragraph {
    /// Creates a paragraph.
    #[must_use]
    pub fn new(text: InlineText) -> Self {
        Self { text }
    }

    /// The paragraph's text.
    #[must_use]
    pub fn text(&self) -> &InlineText {
        &self.text
    }
}

/// A section: a heading and everything beneath it.
///
/// A section owns its nested content rather than sitting flat in a list, so the
/// subtree a theme override applies to is the subtree the type already
/// describes. Theme resolution then needs no separate notion of scope.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    heading: InlineText,
    level: HeadingLevel,
    theme: Option<ThemeId>,
    body: Vec<Block>,
}

impl Section {
    /// Creates a section with no theme override and no content.
    #[must_use]
    pub fn new(heading: InlineText, level: HeadingLevel) -> Self {
        Self {
            heading,
            level,
            theme: None,
            body: Vec::new(),
        }
    }

    /// Declares the theme this section and its subtree render under.
    #[must_use]
    pub fn with_theme(mut self, theme: ThemeId) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Appends a block to the section's content.
    #[must_use]
    pub fn with_block(mut self, block: Block) -> Self {
        self.body.push(block);
        self
    }

    /// The section's heading text.
    #[must_use]
    pub fn heading(&self) -> &InlineText {
        &self.heading
    }

    /// How deeply the heading is nested.
    #[must_use]
    pub fn level(&self) -> HeadingLevel {
        self.level
    }

    /// The theme this section declares, if any.
    #[must_use]
    pub fn theme(&self) -> Option<&ThemeId> {
        self.theme.as_ref()
    }

    /// The section's content.
    #[must_use]
    pub fn body(&self) -> &[Block] {
        &self.body
    }
}

/// A unit of document content.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// A section and everything nested beneath it.
    Section(Section),
    /// A paragraph of body text.
    Paragraph(Paragraph),
}

/// A whole document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    title: Option<InlineText>,
    body: Vec<Block>,
}

impl Document {
    /// Creates an empty, untitled document.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the document title.
    #[must_use]
    pub fn with_title(mut self, title: InlineText) -> Self {
        self.title = Some(title);
        self
    }

    /// Appends a block to the document body.
    #[must_use]
    pub fn with_block(mut self, block: Block) -> Self {
        self.body.push(block);
        self
    }

    /// The document title, if it has one.
    #[must_use]
    pub fn title(&self) -> Option<&InlineText> {
        self.title.as_ref()
    }

    /// The document's top-level content.
    #[must_use]
    pub fn body(&self) -> &[Block] {
        &self.body
    }

    /// Whether the document has neither a title nor any content.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.body.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> InlineText {
        InlineText::new(value)
    }

    fn level(value: u8) -> HeadingLevel {
        HeadingLevel::new(value).unwrap()
    }

    #[test]
    fn a_document_carries_a_title_and_content() {
        let document = Document::new()
            .with_title(text("Report"))
            .with_block(Block::Paragraph(Paragraph::new(text("Opening words."))));

        assert_eq!(document.title().unwrap().as_str(), "Report");
        assert_eq!(document.body().len(), 1);
    }

    #[test]
    fn a_new_document_is_empty() {
        assert!(Document::new().is_empty());
    }

    #[test]
    fn a_section_nests_a_subsection_beneath_it() {
        let subsection = Section::new(text("Details"), level(2))
            .with_block(Block::Paragraph(Paragraph::new(text("Fine print."))));
        let section =
            Section::new(text("Overview"), level(1)).with_block(Block::Section(subsection));

        let Block::Section(nested) = &section.body()[0] else {
            panic!("expected the subsection to be nested inside the section");
        };
        assert_eq!(nested.heading().as_str(), "Details");
        assert_eq!(nested.level().get(), 2);
    }

    #[test]
    fn a_section_declares_no_theme_unless_told_to() {
        assert!(Section::new(text("Plain"), level(1)).theme().is_none());
    }

    #[test]
    fn a_section_remembers_the_theme_it_declares() {
        let section =
            Section::new(text("Appendix"), level(1)).with_theme(ThemeId::new("wide").unwrap());

        assert_eq!(section.theme().unwrap().as_str(), "wide");
    }

    #[test]
    fn heading_levels_span_one_to_six() {
        for value in HeadingLevel::MIN..=HeadingLevel::MAX {
            assert_eq!(HeadingLevel::new(value).unwrap().get(), value);
        }
    }

    #[test]
    fn a_heading_level_of_zero_is_rejected() {
        assert_eq!(
            HeadingLevel::new(0),
            Err(InvalidHeadingLevel { level: 0 }),
            "level 0 is the document title, which is not a section heading"
        );
    }

    #[test]
    fn a_heading_nested_too_deeply_is_rejected() {
        let error = HeadingLevel::new(HeadingLevel::MAX + 1).unwrap_err();

        assert!(
            error.to_string().contains('7'),
            "message must report the offending level, got: {error}"
        );
    }

    #[test]
    fn inline_text_is_kept_exactly_as_written() {
        let source = "a #set page(width: 1cm) b $x$ \\ c";

        assert_eq!(
            text(source).as_str(),
            source,
            "content must not be altered on the way in; escaping happens at emission"
        );
    }

    #[test]
    fn empty_inline_text_reports_itself_as_empty() {
        assert!(text("").is_empty());
        assert!(!text(" ").is_empty());
    }
}
