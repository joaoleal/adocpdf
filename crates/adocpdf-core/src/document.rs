//! The document model: the constructs this renderer understands.
//!
//! Deliberately small. It covers the document title, nested sections,
//! paragraphs and inline text — the set the current specification names as
//! supported — and nothing else. Constructs outside it are reported as skipped
//! by the parser adapter rather than represented here as a half-measure.

use std::error::Error;
use std::fmt;

use crate::theme::ThemeId;

/// A presentation applied to a run of inline content.
///
/// The set is closed, and deliberately so: it is exactly the inline formatting
/// the AsciiDoc language defines and this renderer honours. A style that is not
/// here cannot be represented, which is what stops an unsupported construct
/// being smuggled through as a half-measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlineStyle {
    /// Bold. AsciiDoc calls this *strong*.
    Strong,
    /// Italic. AsciiDoc calls this *emphasis*.
    Emphasis,
    /// Fixed-width, for code and literal text.
    Monospace,
    /// Raised above the baseline.
    Superscript,
    /// Lowered below the baseline.
    Subscript,
    /// Marked for attention.
    Highlight,
}

/// A piece of inline content.
///
/// Styles nest rather than combine: text that is both bold and italic is a
/// [`InlineNode::Styled`] inside another, which is the shape the upstream
/// parser hands over, since it renders the inner content before the outer.
/// Representing a combination as a set on one node would mean flattening that
/// structure on the way in and inventing it again on the way out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineNode {
    /// Text with no internal structure, exactly as it appeared in the source.
    Text(String),
    /// Content set under one presentation.
    Styled {
        /// How the content is presented.
        style: InlineStyle,
        /// What the presentation applies to.
        children: Vec<Self>,
    },
    /// A break the author asked for, as opposed to one the layout engine
    /// chooses when it fills a line.
    LineBreak,
}

impl InlineNode {
    /// Creates a text node.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Creates a styled node around some content.
    #[must_use]
    pub fn styled(style: InlineStyle, children: Vec<Self>) -> Self {
        Self::Styled { style, children }
    }

    /// Appends this node's text to `out`, ignoring presentation.
    ///
    /// A hard line break contributes a newline, because a caller flattening
    /// inline content wants the text the reader would see, and the reader sees
    /// a break there.
    pub fn write_plain_text(&self, out: &mut String) {
        match self {
            Self::Text(text) => out.push_str(text),
            Self::Styled { children, .. } => {
                for child in children {
                    child.write_plain_text(out);
                }
            }
            Self::LineBreak => out.push('\n'),
        }
    }
}

/// A run of inline content: text, and the presentations applied to parts of it.
///
/// Text is stored exactly as it appeared in the source. It is *content*, so it
/// is never restricted to an alphabet the way an identifier or a family name
/// is; instead it passes through the escaper on its way to the output. Storing
/// it verbatim is what lets that escaping be the single chokepoint.
///
/// Structure is stored beside the text rather than encoded into it. A
/// representation that kept markers inside the string would put the escaper
/// back in the business of telling content apart from instructions, which is
/// exactly the distinction it exists to make unnecessary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InlineText(Vec<InlineNode>);

impl InlineText {
    /// Creates inline text from a run of source text with no structure.
    ///
    /// Empty text yields no nodes at all rather than one empty node, so that
    /// two ways of spelling "nothing" do not compare unequal.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        if text.is_empty() {
            Self(Vec::new())
        } else {
            Self(vec![InlineNode::Text(text)])
        }
    }

    /// Creates inline text from already-structured content.
    #[must_use]
    pub fn from_nodes(nodes: Vec<InlineNode>) -> Self {
        Self(nodes)
    }

    /// The content, in document order.
    #[must_use]
    pub fn nodes(&self) -> &[InlineNode] {
        &self.0
    }

    /// The text a reader would see, with every presentation discarded.
    ///
    /// This is what the reporting paths want — a skipped-construct message
    /// names text, not typography — and what a caller comparing against source
    /// text wants. It is never what the emitter wants: emitting this would
    /// throw away the structure the model exists to carry.
    #[must_use]
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for node in &self.0 {
            node.write_plain_text(&mut out);
        }
        out
    }

    /// Whether there is no text at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty() || self.plain_text().is_empty()
    }
}

impl From<Vec<InlineNode>> for InlineText {
    fn from(nodes: Vec<InlineNode>) -> Self {
        Self::from_nodes(nodes)
    }
}

impl fmt::Display for InlineText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.plain_text())
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

/// What kind of verbatim block this is.
///
/// The three differ in what the author meant, not in how they are set: all
/// three preserve their content exactly and are set in the monospace face. The
/// distinction is kept because it is the author's, and because a later change
/// that highlights source code will need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerbatimKind {
    /// A literal block: text to be shown as typed.
    Literal,
    /// A listing block: output, a transcript, or unlabelled code.
    Listing,
    /// A source block: code in a named language.
    Source,
}

/// Content preserved exactly as written.
///
/// The content is a plain string rather than [`InlineText`] because no
/// substitution is applied to it. Formatting characters inside a verbatim block
/// are content, and giving it inline structure would be a way to lose that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verbatim {
    kind: VerbatimKind,
    content: String,
}

impl Verbatim {
    /// Creates a verbatim block.
    #[must_use]
    pub fn new(kind: VerbatimKind, content: impl Into<String>) -> Self {
        Self {
            kind,
            content: content.into(),
        }
    }

    /// What kind of block it is.
    #[must_use]
    pub const fn kind(&self) -> VerbatimKind {
        self.kind
    }

    /// The content, exactly as written.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// Which admonition this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdmonitionKind {
    /// Something worth knowing.
    Note,
    /// Advice.
    Tip,
    /// Something the reader must not miss.
    Important,
    /// Something that could go wrong.
    Caution,
    /// Something that will go wrong.
    Warning,
}

impl AdmonitionKind {
    /// The label a reader sees.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Note => "NOTE",
            Self::Tip => "TIP",
            Self::Important => "IMPORTANT",
            Self::Caution => "CAUTION",
            Self::Warning => "WARNING",
        }
    }
}

/// A passage set apart from the flow of the text and labelled.
#[derive(Debug, Clone, PartialEq)]
pub struct Admonition {
    kind: AdmonitionKind,
    body: Vec<Block>,
}

impl Admonition {
    /// Creates an admonition.
    #[must_use]
    pub fn new(kind: AdmonitionKind, body: Vec<Block>) -> Self {
        Self { kind, body }
    }

    /// Which admonition it is.
    #[must_use]
    pub const fn kind(&self) -> AdmonitionKind {
        self.kind
    }

    /// Its content.
    #[must_use]
    pub fn body(&self) -> &[Block] {
        &self.body
    }
}

/// Whether a quotation is prose or verse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuotationKind {
    /// Prose, which the layout engine may reflow.
    Quote,
    /// Verse, whose line breaks are the author's and must be kept.
    Verse,
}

/// A quotation, with whatever attribution the source supplied.
#[derive(Debug, Clone, PartialEq)]
pub struct Quotation {
    kind: QuotationKind,
    attribution: Option<InlineText>,
    citation: Option<InlineText>,
    body: Vec<Block>,
}

impl Quotation {
    /// Creates a quotation with no attribution.
    #[must_use]
    pub fn new(kind: QuotationKind, body: Vec<Block>) -> Self {
        Self {
            kind,
            attribution: None,
            citation: None,
            body,
        }
    }

    /// Names who is being quoted.
    #[must_use]
    pub fn with_attribution(mut self, attribution: InlineText) -> Self {
        self.attribution = Some(attribution);
        self
    }

    /// Names the work being quoted from.
    #[must_use]
    pub fn with_citation(mut self, citation: InlineText) -> Self {
        self.citation = Some(citation);
        self
    }

    /// Whether it is prose or verse.
    #[must_use]
    pub const fn kind(&self) -> QuotationKind {
        self.kind
    }

    /// Who is being quoted, if the source said.
    #[must_use]
    pub fn attribution(&self) -> Option<&InlineText> {
        self.attribution.as_ref()
    }

    /// The work being quoted from, if the source said.
    #[must_use]
    pub fn citation(&self) -> Option<&InlineText> {
        self.citation.as_ref()
    }

    /// The quoted content.
    #[must_use]
    pub fn body(&self) -> &[Block] {
        &self.body
    }
}

/// A container that holds other blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContainerKind {
    /// An example, set apart from the text around it.
    Example,
    /// A sidebar, set apart more strongly still.
    Sidebar,
    /// An open block, which groups without a presentation of its own.
    Open,
}

/// A group of blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    kind: ContainerKind,
    body: Vec<Block>,
}

impl Container {
    /// Creates a container.
    #[must_use]
    pub fn new(kind: ContainerKind, body: Vec<Block>) -> Self {
        Self { kind, body }
    }

    /// What kind of container it is.
    #[must_use]
    pub const fn kind(&self) -> ContainerKind {
        self.kind
    }

    /// Its content.
    #[must_use]
    pub fn body(&self) -> &[Block] {
        &self.body
    }
}

/// What kind of list this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListKind {
    /// Items marked but not numbered.
    Unordered,
    /// Items numbered by their position.
    Ordered,
    /// Terms, each with a description.
    Description,
}

/// One item of a list.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    term: Option<InlineText>,
    body: Vec<Block>,
}

impl ListItem {
    /// Creates an item with no term.
    #[must_use]
    pub fn new(body: Vec<Block>) -> Self {
        Self { term: None, body }
    }

    /// Gives the item the term it describes.
    ///
    /// Only a description list has terms; the others carry a marker or a
    /// number, which is the list's business rather than the item's.
    #[must_use]
    pub fn with_term(mut self, term: InlineText) -> Self {
        self.term = Some(term);
        self
    }

    /// The term this item describes, if it has one.
    #[must_use]
    pub fn term(&self) -> Option<&InlineText> {
        self.term.as_ref()
    }

    /// The item's content.
    #[must_use]
    pub fn body(&self) -> &[Block] {
        &self.body
    }
}

/// A list of items.
#[derive(Debug, Clone, PartialEq)]
pub struct List {
    kind: ListKind,
    items: Vec<ListItem>,
}

impl List {
    /// Creates a list.
    #[must_use]
    pub fn new(kind: ListKind, items: Vec<ListItem>) -> Self {
        Self { kind, items }
    }

    /// What kind of list it is.
    #[must_use]
    pub const fn kind(&self) -> ListKind {
        self.kind
    }

    /// Its items, in order.
    #[must_use]
    pub fn items(&self) -> &[ListItem] {
        &self.items
    }
}

/// A break in the flow of the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BreakKind {
    /// A visible division between passages.
    Thematic,
    /// The rest of the document starts on a new page.
    Page,
}

/// A unit of document content.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// A section and everything nested beneath it.
    Section(Section),
    /// A paragraph of body text.
    Paragraph(Paragraph),
    /// Content preserved exactly as written.
    Verbatim(Verbatim),
    /// A labelled passage, set apart from the text.
    Admonition(Admonition),
    /// A quotation, in prose or in verse.
    Quotation(Quotation),
    /// A group of blocks.
    Container(Container),
    /// A list of items.
    List(List),
    /// A thematic or page break.
    Break(BreakKind),
    /// A block with a title above it.
    ///
    /// A wrapper rather than a field on every block that can carry one: a
    /// title behaves the same way whatever it titles, and repeating the field
    /// would invite the blocks to disagree about it.
    Titled {
        /// The title.
        title: InlineText,
        /// What it titles.
        block: Box<Self>,
    },
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

        assert_eq!(document.title().unwrap().plain_text(), "Report");
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
        assert_eq!(nested.heading().plain_text(), "Details");
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
    fn a_styled_node_nests_another_inside_it() {
        let node = InlineNode::styled(
            InlineStyle::Strong,
            vec![
                InlineNode::text("bold "),
                InlineNode::styled(InlineStyle::Emphasis, vec![InlineNode::text("and italic")]),
            ],
        );

        let InlineNode::Styled { style, children } = &node else {
            panic!("expected a styled node");
        };
        assert_eq!(*style, InlineStyle::Strong);
        assert_eq!(children.len(), 2);

        let InlineNode::Styled { style: inner, .. } = &children[1] else {
            panic!("expected the nested span to survive");
        };
        assert_eq!(
            *inner,
            InlineStyle::Emphasis,
            "nesting carries both presentations"
        );
    }

    #[test]
    fn plain_text_flattens_a_nested_span() {
        let node = InlineNode::styled(
            InlineStyle::Strong,
            vec![
                InlineNode::text("a"),
                InlineNode::styled(InlineStyle::Emphasis, vec![InlineNode::text("b")]),
            ],
        );

        let mut out = String::new();
        node.write_plain_text(&mut out);

        assert_eq!(out, "ab", "flattening reports text, not presentation");
    }

    #[test]
    fn a_hard_line_break_flattens_to_a_newline() {
        let mut out = String::new();
        InlineNode::LineBreak.write_plain_text(&mut out);

        assert_eq!(
            out, "\n",
            "a reader sees a break there, so flattened text has one"
        );
    }

    #[test]
    fn inline_text_is_kept_exactly_as_written() {
        let source = "a #set page(width: 1cm) b $x$ \\ c";

        assert_eq!(
            text(source).plain_text(),
            source,
            "content must not be altered on the way in; escaping happens at emission"
        );
    }

    #[test]
    fn inline_text_flattens_its_structure_to_plain_text() {
        let structured = InlineText::from_nodes(vec![
            InlineNode::text("a "),
            InlineNode::styled(
                InlineStyle::Strong,
                vec![
                    InlineNode::text("b"),
                    InlineNode::styled(InlineStyle::Emphasis, vec![InlineNode::text("c")]),
                ],
            ),
            InlineNode::text("."),
        ]);

        assert_eq!(
            structured.plain_text(),
            "a bc.",
            "the reporting paths want the text a reader sees, not its typography"
        );
    }

    #[test]
    fn plain_source_text_becomes_a_single_node() {
        let text = InlineText::new("just words");

        assert_eq!(text.nodes(), [InlineNode::Text("just words".to_owned())]);
    }

    #[test]
    fn empty_inline_text_reports_itself_as_empty() {
        assert!(text("").is_empty());
        assert!(!text(" ").is_empty());
    }
}
