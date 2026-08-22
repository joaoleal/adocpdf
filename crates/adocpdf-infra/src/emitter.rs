//! Turning a layout plan into engine markup.
//!
//! Two rules govern everything here:
//!
//! 1. Text from the source reaches the output only through
//!    [`crate::markup::string_literal`]. Nothing else interpolates it.
//! 2. Structural instructions are built only from validated model values — a
//!    [`Length`](adocpdf_core::length::Length) formats as a number, a font
//!    family has already been restricted to a closed alphabet. So the set of
//!    characters that can appear in a structural position is finite and known.
//!
//! Everything is emitted in code mode, as function calls, rather than as
//! markup. Markup is whitespace- and position-sensitive — a `-` starting a line
//! makes a list, an indent can nest a block — and generated text has no
//! business depending on that.

use std::fmt::Write as _;

use adocpdf_core::document::{
    AdmonitionKind, BreakKind, ContainerKind, HeadingLevel, InlineNode, InlineStyle, InlineText,
    ListKind, QuotationKind, Verbatim,
};
use adocpdf_core::geometry::PageGeometry;
use adocpdf_core::theme::{Theme, ThemeTransition};
use adocpdf_core::typography::Typography;
use adocpdf_domain::document_plan::{GroupKind, LayoutPlan, PlanItem};

use crate::markup::string_literal;

/// How much larger than body text the document title is set.
const TITLE_SCALE: f64 = 1.8;

/// Renders a plan as engine source.
#[must_use]
pub fn emit(plan: &LayoutPlan) -> String {
    let mut emitter = Emitter {
        out: String::new(),
        body_size_points: plan.initial_theme().typography().size().points(),
        monospace_family: plan
            .initial_theme()
            .typography()
            .monospace_family()
            .as_str()
            .to_owned(),
    };

    emitter.page(plan.initial_theme().geometry());
    emitter.text_style(plan.initial_theme().typography());

    for item in plan.items() {
        emitter.item(item);
    }

    emitter.out
}

/// Accumulates markup while tracking the style currently in effect.
///
/// The title is set relative to body size, so emission needs to know what body
/// size a theme change left behind. Keeping that here means the plan does not
/// have to carry resolved sizes for every item.
struct Emitter {
    out: String,
    body_size_points: f64,
    /// The family monospaced content is set in, from the theme in effect.
    monospace_family: String,
}

impl Emitter {
    fn item(&mut self, item: &PlanItem) {
        match item {
            PlanItem::Title(text) => {
                let size = points(self.body_size_points * TITLE_SCALE);
                let _ = writeln!(
                    self.out,
                    "#align(center, text(size: {size}, weight: \"bold\", {}))",
                    self.inline(text)
                );
                let _ = writeln!(self.out, "#v({})", points(self.body_size_points));
            }
            PlanItem::Heading { text, level } => {
                // The size is named rather than left to the engine, which sets
                // every level below the second at body size — so a document
                // with three depths of section arrived on the page with three
                // headings a reader could not tell apart, and the structure
                // was only recoverable from the source.
                let _ = writeln!(
                    self.out,
                    "#heading(level: {}, text(size: {}, {}))",
                    level.get(),
                    points(self.heading_size_points(level.get())),
                    self.inline(text)
                );
            }
            PlanItem::Paragraph(text) => {
                let _ = writeln!(self.out, "#par({})", self.inline(text));
            }
            PlanItem::Verbatim(verbatim) => self.verbatim(verbatim),
            PlanItem::Break(kind) => self.block_break(*kind),
            PlanItem::BlockTitle(text) => {
                // Set apart from body text and from a heading alike: a block
                // title names the block below it and takes no part in the
                // section hierarchy.
                let _ = writeln!(
                    self.out,
                    "#block(above: {}, below: {}, text(style: \"italic\", weight: \"bold\", {}))",
                    points(self.body_size_points * 0.6),
                    points(self.body_size_points * 0.2),
                    self.inline(text)
                );
            }
            PlanItem::Group { kind, children } => self.group(kind, children),
            PlanItem::ThemeChange { theme, transition } => self.theme_change(theme, *transition),
        }
    }

    /// The size a heading of `level` is set at.
    ///
    /// A fixed step down from the level above, floored so that the deepest
    /// level is still larger than body text: a heading that has shrunk to the
    /// size of the prose beneath it has stopped being a heading. The scale
    /// belongs to the emitter rather than to the theme for now — the theme has
    /// no heading settings, and giving it some is a change to the model in
    /// every crate rather than a fix to how a page is set.
    fn heading_size_points(&self, level: u8) -> f64 {
        // Anchored at the deepest level and stepped upwards, rather than
        // stepped down from the first and floored. A floor is what produces two
        // levels of the same size, which is the defect this replaced.
        const DEEPEST: f64 = 1.10;
        const STEP: f64 = 0.09;

        let from_deepest = f64::from(HeadingLevel::MAX.saturating_sub(level));
        self.body_size_points * from_deepest.mul_add(STEP, DEEPEST)
    }

    /// Emits content that must reach the page exactly as written.
    ///
    /// `raw` is the engine's own verbatim element: it keeps every space and
    /// line break, and it does not reflow. The text still goes through
    /// [`string_literal`], so the escaping story is unchanged — what differs is
    /// that the engine is told not to interpret the result.
    ///
    /// The face comes from the show rule emitted with the theme's other
    /// typography, not from here: `raw` carries a show-set rule of its own that
    /// hardcodes a family, and a show-set beats an enclosing `text(font: …)`.
    fn verbatim(&mut self, verbatim: &Verbatim) {
        let _ = writeln!(
            self.out,
            "#block(width: 100%, inset: {}, raw(block: true, lang: none, {}))",
            points(self.body_size_points * 0.5),
            string_literal(verbatim.content()),
        );
    }

    fn block_break(&mut self, kind: BreakKind) {
        let _ = match kind {
            BreakKind::Page => writeln!(self.out, "#pagebreak(weak: true)"),
            BreakKind::Thematic => writeln!(
                self.out,
                "#align(center, block(above: {}, below: {}, line(length: 30%)))",
                points(self.body_size_points),
                points(self.body_size_points),
            ),
        };
    }

    /// Emits a group and everything inside it.
    fn group(&mut self, kind: &GroupKind, children: &[PlanItem]) {
        match kind {
            GroupKind::Admonition(admonition) => self.admonition(*admonition, children),
            GroupKind::Quotation {
                kind,
                attribution,
                citation,
            } => self.quotation(*kind, attribution.as_ref(), citation.as_ref(), children),
            GroupKind::Container(container) => self.container(*container, children),
            GroupKind::List(list) => self.list(*list, children),
            // Reached only through `list`, which knows which list the item is
            // in and so which element it becomes.
            GroupKind::ListItem { .. } => self.children(children),
        }
    }

    fn admonition(&mut self, admonition: AdmonitionKind, children: &[PlanItem]) {
        {
            {
                // The label and the rule are what set it apart. There is no
                // icon: an icon would need a face to draw it from, and the
                // repertoire is fixed to the embedded families.
                let _ = writeln!(
                    self.out,
                    "#block(width: 100%, inset: (left: {}, top: {}, bottom: {}), \
                     stroke: (left: 2pt + gray), text(weight: \"bold\", {}) + linebreak() + [",
                    points(self.body_size_points),
                    points(self.body_size_points * 0.4),
                    points(self.body_size_points * 0.4),
                    string_literal(admonition.label()),
                );
                self.children(children);
                let _ = writeln!(self.out, "])");
            }
        }
    }

    fn quotation(
        &mut self,
        kind: QuotationKind,
        attribution: Option<&InlineText>,
        citation: Option<&InlineText>,
        children: &[PlanItem],
    ) {
        {
            {
                let _ = writeln!(
                    self.out,
                    "#block(width: 100%, inset: (left: {}, right: {}), [",
                    points(self.body_size_points * 2.0),
                    points(self.body_size_points * 2.0),
                );
                if kind == QuotationKind::Verse {
                    // Verse keeps the author's line breaks, so the engine must
                    // not fill lines for it.
                    let _ = writeln!(
                        self.out,
                        "#set par(leading: {});",
                        points(self.body_size_points * 0.4)
                    );
                }
                self.children(children);
                self.attribution(attribution, citation);
                let _ = writeln!(self.out, "])");
            }
        }
    }

    fn container(&mut self, container: ContainerKind, children: &[PlanItem]) {
        {
            {
                let _ = match container {
                    ContainerKind::Sidebar => writeln!(
                        self.out,
                        "#block(width: 100%, inset: {}, fill: luma(240), stroke: 0.5pt + gray, [",
                        points(self.body_size_points * 0.8),
                    ),
                    ContainerKind::Example => writeln!(
                        self.out,
                        "#block(width: 100%, inset: {}, stroke: 0.5pt + gray, [",
                        points(self.body_size_points * 0.8),
                    ),
                    // An open block groups without a presentation of its own,
                    // so it emits a container with nothing set on it.
                    ContainerKind::Open => writeln!(self.out, "#block(width: 100%, ["),
                };
                self.children(children);
                let _ = writeln!(self.out, "])");
            }
        }
    }

    /// Emits a list through the engine's own list element.
    ///
    /// The marker is not written as text. Each item's content is passed as an
    /// argument to `list.item`, `enum.item` or `terms.item`, and the engine
    /// places the marker, sets the hanging indent for a wrapped item, and
    /// indents a nested level.
    ///
    /// Writing the marker ourselves is what the emitter used to do, and it went
    /// wrong twice over. A bullet followed by the item's paragraph put the
    /// marker on a line of its own, because a paragraph is block level and
    /// begins a new line. An ordered item avoided that only because `1. ` was
    /// read back by the engine as its own enumeration syntax — emitter output
    /// being reinterpreted as markup, which is exactly what this file's
    /// string-literal discipline exists to prevent everywhere else.
    fn list(&mut self, list: ListKind, items: &[PlanItem]) {
        let element = match list {
            ListKind::Unordered => "list",
            ListKind::Ordered => "enum",
            ListKind::Description => "terms",
        };

        let _ = writeln!(self.out, "#{element}(");
        for item in items {
            self.list_item(list, item);
        }
        let _ = writeln!(self.out, ")");
    }

    /// Emits one item of a list of `list`'s kind.
    fn list_item(&mut self, list: ListKind, item: &PlanItem) {
        let PlanItem::Group {
            kind: GroupKind::ListItem { term, position },
            children,
        } = item
        else {
            // A list's children are its items; anything else is a planning
            // error rather than a document the author wrote, so emit it plainly
            // instead of inventing an item for it.
            self.item(item);
            return;
        };

        match list {
            ListKind::Unordered => {
                let _ = write!(self.out, "list.item(");
            }
            ListKind::Ordered => {
                let _ = write!(self.out, "enum.item({position}, ");
            }
            ListKind::Description => {
                let _ = write!(
                    self.out,
                    "terms.item({}, ",
                    term.as_ref()
                        .map_or_else(|| string_literal(""), |text| self.inline(text)),
                );
            }
        }

        self.item_body(children);
        let _ = writeln!(self.out, "),");
    }

    /// Emits a list item's content as one content value.
    ///
    /// An item holding a single paragraph emits that paragraph's inline content
    /// directly, with no block around it, so the marker and the first line of
    /// the text share a line. Anything more — a nested list, a continuation —
    /// goes inside a content block, where block-level children are allowed and
    /// the first of them still starts beside the marker.
    fn item_body(&mut self, children: &[PlanItem]) {
        if let [PlanItem::Paragraph(text)] = children {
            let _ = write!(self.out, "{}", self.inline(text));
            return;
        }

        let _ = writeln!(self.out, "[");
        for (index, child) in children.iter().enumerate() {
            // The item's own first paragraph belongs beside the marker; a later
            // one is a continuation and is a paragraph in its own right.
            match (index, child) {
                (0, PlanItem::Paragraph(text)) => {
                    // Parenthesised, and prefixed: this is markup mode, where a
                    // bare expression is literal text — the quotes of a string
                    // literal included, which the engine then curls into smart
                    // quotes. The parentheses keep a concatenation together,
                    // since `#a + b` in markup is `#a` followed by the text
                    // ` + b`.
                    let _ = writeln!(self.out, "#({})", self.inline(text));
                }
                _ => self.item(child),
            }
        }
        let _ = write!(self.out, "]");
    }

    /// Emits the attribution line under a quotation.
    fn attribution(&mut self, attribution: Option<&InlineText>, citation: Option<&InlineText>) {
        let parts: Vec<String> = [attribution, citation]
            .into_iter()
            .flatten()
            .map(|text| self.inline(text))
            .collect();

        // No attribution and no citation means no line at all, rather than an
        // empty dash sitting under the quotation.
        if parts.is_empty() {
            return;
        }

        let _ = writeln!(
            self.out,
            "#block(above: {}, text(style: \"italic\", \"\\u{{2014}} \" + {}))",
            points(self.body_size_points * 0.5),
            parts.join(r#" + ", " + "#),
        );
    }

    /// Emits a group's contents.
    fn children(&mut self, children: &[PlanItem]) {
        for child in children {
            self.item(child);
        }
    }

    /// Renders inline content as a content expression, in code mode.
    ///
    /// Every run of text goes through [`string_literal`] and nothing else;
    /// every structural instruction is a function name from this file's own
    /// closed vocabulary. Content values concatenate with `+`, so a sequence
    /// needs no markup mode and no whitespace-sensitive syntax.
    ///
    /// Empty content still has to produce a valid expression, so it produces
    /// an empty string literal rather than nothing at all.
    fn inline(&self, text: &InlineText) -> String {
        let rendered = self.nodes(text.nodes());
        if rendered.is_empty() {
            string_literal("")
        } else {
            rendered
        }
    }

    /// Renders a sequence of nodes as one concatenated content expression.
    fn nodes(&self, nodes: &[InlineNode]) -> String {
        nodes
            .iter()
            .map(|node| self.node(node))
            .collect::<Vec<_>>()
            .join(" + ")
    }

    /// Renders one node.
    fn node(&self, node: &InlineNode) -> String {
        match node {
            InlineNode::Text(text) => string_literal(text),
            InlineNode::LineBreak => "linebreak()".to_owned(),
            InlineNode::Styled { style, children } => {
                let inner = if children.is_empty() {
                    string_literal("")
                } else {
                    self.nodes(children)
                };

                match style {
                    InlineStyle::Strong => format!("strong({inner})"),
                    InlineStyle::Emphasis => format!("emph({inner})"),
                    // Monospace is a face change rather than a semantic mark,
                    // so it names the theme's monospace family directly. The
                    // family has already been restricted to a closed alphabet
                    // by its value object, so it is safe in this position.
                    InlineStyle::Monospace => format!(
                        "text(font: {}, {inner})",
                        string_literal(&self.monospace_family)
                    ),
                    InlineStyle::Superscript => format!("super({inner})"),
                    InlineStyle::Subscript => format!("sub({inner})"),
                    InlineStyle::Highlight => format!("highlight({inner})"),
                }
            }
        }
    }

    fn theme_change(&mut self, theme: &Theme, transition: ThemeTransition) {
        // A page instruction is what makes the engine start a new page, so it
        // is emitted only when the plan says a break is intended. Emitting it
        // for a typography-only change would break the page against the
        // specification.
        if transition.forces_page_break() {
            self.page(theme.geometry());
        }
        self.text_style(theme.typography());
    }

    fn page(&mut self, geometry: &PageGeometry) {
        let margins = geometry.margins();
        let _ = writeln!(
            self.out,
            "#set page(width: {}, height: {}, margin: (top: {}, right: {}, bottom: {}, left: {}))",
            points(geometry.width().points()),
            points(geometry.height().points()),
            points(margins.top.points()),
            points(margins.right.points()),
            points(margins.bottom.points()),
            points(margins.left.points()),
        );
    }

    fn text_style(&mut self, typography: &Typography) {
        self.body_size_points = typography.size().points();
        self.monospace_family.clear();
        self.monospace_family
            .push_str(typography.monospace_family().as_str());

        // `lang` is emitted only when the theme names one. Hyphenation depends
        // on it: the engine hyphenates only when the language is known.
        let language = typography.language().map_or_else(String::new, |tag| {
            format!(", lang: {}", string_literal(tag.as_str()))
        });

        let _ = writeln!(
            self.out,
            "#set text(font: {}, size: {}{}, costs: (orphan: {}%, widow: {}%))",
            string_literal(typography.family().as_str()),
            points(typography.size().points()),
            language,
            typography.avoidance().percent(),
            typography.avoidance().percent(),
        );

        // Verbatim content needs a rule of its own. The engine's `raw` element
        // carries a show-set that names its own monospace family, and a
        // show-set beats the enclosing text style — so without this line a
        // theme's monospace family reached inline monospaced text and was
        // silently ignored by every listing and literal block. It matched the
        // engine's default face by coincidence, which is why nothing looked
        // wrong.
        let _ = writeln!(
            self.out,
            "#show raw: set text(font: {})",
            string_literal(&self.monospace_family),
        );

        // `linebreaks: "optimized"` is asked for explicitly rather than being
        // reached by turning justification on. The engine selects its optimal
        // breaker only when `justify` is set and otherwise falls back to a
        // greedy first-fit pass, so leaving this to `justify` would mean an
        // unjustified document silently gets the worse algorithm — which is
        // what every page this renderer produced before this line existed got.
        let _ = writeln!(
            self.out,
            "#set par(leading: {}, linebreaks: \"optimized\", justify: {})",
            points(typography.leading().points()),
            typography.is_justified(),
        );
    }
}

/// Formats a length as an engine measurement.
///
/// Rust's shortest-round-trip formatting for `f64` is deterministic, so the same
/// length always produces the same characters — which byte-identical output
/// depends on.
fn points(value: f64) -> String {
    format!("{value}pt")
}

#[cfg(test)]
mod tests {
    use adocpdf_core::document::{Block, Document, HeadingLevel, InlineText, Paragraph, Section};
    use adocpdf_core::geometry::{Margins, PageGeometry};
    use adocpdf_core::length::Length;
    use adocpdf_core::theme::{ThemeId, ThemeSet, built_in_default_theme};
    use adocpdf_core::typography::{FontFamily, Typography};
    use adocpdf_domain::document_plan::plan_document;

    use super::*;

    fn wide_theme() -> Theme {
        Theme::new(
            PageGeometry::new(
                Length::from_millimeters(420.0).unwrap(),
                Length::from_millimeters(297.0).unwrap(),
                Margins::uniform(Length::from_millimeters(20.0).unwrap()),
            )
            .unwrap(),
            built_in_default_theme().typography().clone(),
        )
    }

    fn restyled_theme() -> Theme {
        Theme::new(
            *built_in_default_theme().geometry(),
            Typography::new(
                FontFamily::new("DejaVu Sans").unwrap(),
                Length::from_points(13.0).unwrap(),
                Length::from_points(18.0).unwrap(),
            ),
        )
    }

    fn themes() -> ThemeSet {
        ThemeSet::default()
            .with(ThemeId::new("wide").unwrap(), wide_theme())
            .with(ThemeId::new("restyled").unwrap(), restyled_theme())
    }

    fn emit_document(document: &Document) -> String {
        emit(&plan_document(document, &themes()).unwrap())
    }

    /// Counts page instructions the engine will actually act on.
    ///
    /// Every instruction the emitter produces starts its own line, so anything
    /// mid-line is inside a string literal and is inert content. Counting raw
    /// substrings would conflate the two and let an injection look like a pass.
    fn page_instructions(markup: &str) -> usize {
        markup
            .lines()
            .filter(|line| line.starts_with("#set page("))
            .count()
    }

    fn themed_section(theme: &str) -> Block {
        Block::Section(
            Section::new(InlineText::new("Section"), HeadingLevel::new(1).unwrap())
                .with_theme(ThemeId::new(theme).unwrap())
                .with_block(Block::Paragraph(Paragraph::new(InlineText::new("Body.")))),
        )
    }

    #[test]
    fn a_plain_document_sets_page_and_text_once() {
        let document = Document::new()
            .with_title(InlineText::new("Report"))
            .with_block(Block::Paragraph(Paragraph::new(InlineText::new("Body."))));

        let markup = emit_document(&document);

        assert_eq!(
            page_instructions(&markup),
            1,
            "an unthemed document needs exactly one page instruction, got:\n{markup}"
        );
        assert!(markup.contains(r#"#par("Body.")"#), "got:\n{markup}");
    }

    #[test]
    fn a_geometry_change_emits_a_second_page_instruction() {
        let document = Document::new().with_block(themed_section("wide"));

        let markup = emit_document(&document);

        assert_eq!(
            page_instructions(&markup),
            2,
            "the page instruction is what starts the new page, got:\n{markup}"
        );
    }

    #[test]
    fn a_typography_change_emits_no_page_instruction() {
        let document = Document::new().with_block(themed_section("restyled"));

        let markup = emit_document(&document);

        assert_eq!(
            page_instructions(&markup),
            1,
            "restyling must not break the page, got:\n{markup}"
        );
        assert!(markup.contains("size: 13pt"), "got:\n{markup}");
    }

    #[test]
    fn a_heading_carries_its_level() {
        let document = Document::new().with_block(Block::Section(Section::new(
            InlineText::new("Overview"),
            HeadingLevel::new(2).unwrap(),
        )));

        let markup = emit_document(&document);

        // The level and the size both, because the engine sets every level
        // below the second at body size and the level alone would not reach
        // the page as anything a reader could see.
        assert!(
            markup.contains(r"#heading(level: 2, text(size: "),
            "got:\n{markup}"
        );
        assert!(markup.contains("\"Overview\""), "got:\n{markup}");
    }

    #[test]
    fn source_text_reaches_the_output_only_as_a_literal() {
        let attack = r#"") #set page(width: 1cm) #par(""#;
        let document =
            Document::new().with_block(Block::Paragraph(Paragraph::new(InlineText::new(attack))));

        let markup = emit_document(&document);

        assert_eq!(
            page_instructions(&markup),
            1,
            "content must not be able to introduce a page instruction, got:\n{markup}"
        );
    }

    #[test]
    fn a_heading_cannot_smuggle_an_instruction() {
        let document = Document::new().with_block(Block::Section(Section::new(
            InlineText::new(r#"Innocent") #set page(width: 1cm) #heading(level: 1, ""#),
            HeadingLevel::new(1).unwrap(),
        )));

        let markup = emit_document(&document);

        assert_eq!(page_instructions(&markup), 1, "got:\n{markup}");
    }

    #[test]
    fn the_same_plan_always_emits_the_same_characters() {
        let document = Document::new()
            .with_title(InlineText::new("Report"))
            .with_block(themed_section("wide"));

        assert_eq!(
            emit_document(&document),
            emit_document(&document),
            "emission must be deterministic for output to be byte-identical"
        );
    }

    #[test]
    fn the_title_is_set_larger_than_body_text() {
        let document = Document::new().with_title(InlineText::new("Report"));

        let markup = emit_document(&document);

        assert!(
            markup.contains(&format!("size: {}pt", 11.0 * TITLE_SCALE)),
            "got:\n{markup}"
        );
    }

    #[test]
    fn lengths_are_emitted_in_points() {
        assert_eq!(points(11.0), "11pt");
        assert_eq!(points(15.4), "15.4pt");
    }

    fn styled(style: InlineStyle, text: &str) -> InlineText {
        InlineText::from_nodes(vec![InlineNode::styled(
            style,
            vec![InlineNode::text(text)],
        )])
    }

    fn emit_paragraph(text: InlineText) -> String {
        let document = Document::new().with_block(Block::Paragraph(Paragraph::new(text)));
        let plan = plan_document(&document, &ThemeSet::default()).unwrap();
        emit(&plan)
    }

    #[test]
    fn each_style_emits_its_own_engine_function() {
        for (style, expected) in [
            (InlineStyle::Strong, r#"strong("word")"#),
            (InlineStyle::Emphasis, r#"emph("word")"#),
            (InlineStyle::Superscript, r#"super("word")"#),
            (InlineStyle::Subscript, r#"sub("word")"#),
            (InlineStyle::Highlight, r#"highlight("word")"#),
        ] {
            let markup = emit_paragraph(styled(style, "word"));

            assert!(
                markup.contains(expected),
                "expected {expected} for {style:?}, got:\n{markup}"
            );
        }
    }

    #[test]
    fn monospace_names_the_theme_s_monospace_family() {
        let markup = emit_paragraph(styled(InlineStyle::Monospace, "code"));

        assert!(
            markup.contains(r#"text(font: "DejaVu Sans Mono", "code")"#),
            "monospace is a face change, and the face comes from the theme, got:\n{markup}"
        );
    }

    #[test]
    fn a_sequence_of_nodes_is_concatenated() {
        let text = InlineText::from_nodes(vec![
            InlineNode::text("Hello "),
            InlineNode::styled(InlineStyle::Strong, vec![InlineNode::text("world")]),
            InlineNode::text("."),
        ]);

        let markup = emit_paragraph(text);

        assert!(
            markup.contains(r#"#par("Hello " + strong("world") + ".")"#),
            "got:\n{markup}"
        );
    }

    #[test]
    fn nested_styles_nest_their_calls() {
        let text = InlineText::from_nodes(vec![InlineNode::styled(
            InlineStyle::Strong,
            vec![InlineNode::styled(
                InlineStyle::Emphasis,
                vec![InlineNode::text("both")],
            )],
        )]);

        assert!(emit_paragraph(text).contains(r#"strong(emph("both"))"#));
    }

    #[test]
    fn a_hard_line_break_emits_a_break_call() {
        let text = InlineText::from_nodes(vec![
            InlineNode::text("one"),
            InlineNode::LineBreak,
            InlineNode::text("two"),
        ]);

        assert!(
            emit_paragraph(text).contains(r#"#par("one" + linebreak() + "two")"#),
            "a break the author asked for is an instruction, not a newline in a literal"
        );
    }

    #[test]
    fn styled_text_still_reaches_the_output_through_the_escaper() {
        // The injection boundary does not move because the text is inside a
        // style: every run is still a string literal.
        let attack = r#"") #set page(width: 1cm) #text(""#;
        let markup = emit_paragraph(styled(InlineStyle::Strong, attack));

        assert!(
            markup.contains(&string_literal(attack)),
            "the attack text must appear escaped, got:\n{markup}"
        );
        // The attack text does appear in the output — inside a string
        // literal, which is the point. What must not appear is a second page
        // *instruction*, so count instructions rather than substrings.
        let instructions = markup
            .lines()
            .filter(|line| line.starts_with("#set page"))
            .count();
        assert_eq!(
            instructions, 1,
            "only the emitter's own page instruction may appear, got:\n{markup}"
        );
    }

    #[test]
    fn empty_inline_content_still_emits_a_valid_expression() {
        let markup = emit_paragraph(InlineText::default());

        assert!(
            markup.contains(r#"#par("")"#),
            "an empty paragraph must still be a well-formed call, got:\n{markup}"
        );
    }
}
