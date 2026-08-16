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

use adocpdf_core::geometry::PageGeometry;
use adocpdf_core::theme::{Theme, ThemeTransition};
use adocpdf_core::typography::Typography;
use adocpdf_domain::document_plan::{LayoutPlan, PlanItem};

use crate::markup::string_literal;

/// How much larger than body text the document title is set.
const TITLE_SCALE: f64 = 1.8;

/// Renders a plan as engine source.
#[must_use]
pub fn emit(plan: &LayoutPlan) -> String {
    let mut emitter = Emitter {
        out: String::new(),
        body_size_points: plan.initial_theme().typography().size().points(),
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
}

impl Emitter {
    fn item(&mut self, item: &PlanItem) {
        match item {
            PlanItem::Title(text) => {
                let size = points(self.body_size_points * TITLE_SCALE);
                let _ = writeln!(
                    self.out,
                    "#align(center, text(size: {size}, weight: \"bold\", {}))",
                    string_literal(text.as_str())
                );
                let _ = writeln!(self.out, "#v({})", points(self.body_size_points));
            }
            PlanItem::Heading { text, level } => {
                let _ = writeln!(
                    self.out,
                    "#heading(level: {}, {})",
                    level.get(),
                    string_literal(text.as_str())
                );
            }
            PlanItem::Paragraph(text) => {
                let _ = writeln!(self.out, "#par({})", string_literal(text.as_str()));
            }
            PlanItem::ThemeChange { theme, transition } => self.theme_change(theme, *transition),
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

        let _ = writeln!(
            self.out,
            "#set text(font: {}, size: {})",
            string_literal(typography.family().as_str()),
            points(typography.size().points()),
        );
        let _ = writeln!(
            self.out,
            "#set par(leading: {})",
            points(typography.leading().points())
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

        assert!(
            markup.contains(r#"#heading(level: 2, "Overview")"#),
            "got:\n{markup}"
        );
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
}
