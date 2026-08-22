//! Deciding which theme applies where, and flattening the result.
//!
//! Theme resolution is a business rule, so it happens here rather than inside a
//! rendering adapter. What comes out is a [`LayoutPlan`]: a flat sequence in
//! document order, with theme changes already positioned and already classified
//! as page-breaking or not. An adapter consuming it needs no notion of scope,
//! inheritance, or precedence.

use adocpdf_core::document::{
    AdmonitionKind, Block, BreakKind, ContainerKind, Document, HeadingLevel, InlineText, ListKind,
    QuotationKind, Section, Verbatim,
};
use adocpdf_core::theme::{Theme, ThemeSet, ThemeTransition};

use crate::error::DomainError;

/// One step in a laid-out document.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanItem {
    /// The document title.
    Title(InlineText),
    /// A section heading.
    Heading {
        /// The heading text.
        text: InlineText,
        /// How deeply it is nested.
        level: HeadingLevel,
    },
    /// A paragraph of body text.
    Paragraph(InlineText),
    /// Content preserved exactly as written.
    Verbatim(Verbatim),
    /// A break in the flow of the document.
    Break(BreakKind),
    /// A title belonging to the item that follows it.
    BlockTitle(InlineText),
    /// A group of items, set according to what kind of group it is.
    ///
    /// The plan is otherwise flat, and stays flat wherever it can: theme
    /// resolution needs no notion of scope, and flattening is what frees an
    /// adapter from having to model one. Groups are the exception because a
    /// list, an admonition and a sidebar genuinely contain other blocks, and
    /// an adapter given a depth number instead would have to reconstruct the
    /// nesting to emit it — with nothing to catch a depth sequence that made
    /// no sense.
    Group {
        /// What kind of group it is.
        kind: GroupKind,
        /// What it contains, already planned.
        children: Vec<Self>,
    },
    /// Everything after this point is set under a different theme.
    ThemeChange {
        /// The theme now in effect.
        theme: Theme,
        /// What kind of change it is, and so whether a page break follows.
        transition: ThemeTransition,
    },
}

/// What a [`PlanItem::Group`] is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupKind {
    /// A labelled passage set apart from the text.
    Admonition(AdmonitionKind),
    /// A quotation, with whatever attribution the source supplied.
    Quotation {
        /// Whether it is prose or verse.
        kind: QuotationKind,
        /// Who is being quoted.
        attribution: Option<InlineText>,
        /// The work being quoted from.
        citation: Option<InlineText>,
    },
    /// A container holding other blocks.
    Container(ContainerKind),
    /// A list. Its children are its items, each of them an
    /// [`GroupKind::ListItem`] group.
    List(ListKind),
    /// One item of a list.
    ListItem {
        /// The term this item describes, for a description list.
        term: Option<InlineText>,
        /// Which item this is, counting from one, for an ordered list.
        position: usize,
    },
}

/// A document flattened into the order it will be laid out in.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutPlan {
    initial: Theme,
    items: Vec<PlanItem>,
}

impl LayoutPlan {
    /// The theme in effect before the first item.
    #[must_use]
    pub fn initial_theme(&self) -> &Theme {
        &self.initial
    }

    /// The items, in document order.
    #[must_use]
    pub fn items(&self) -> &[PlanItem] {
        &self.items
    }

    /// How many page breaks the theme changes in this plan force.
    ///
    /// Lets an author be told what a theme choice will do before rendering,
    /// which the theming specification requires.
    #[must_use]
    pub fn forced_page_breaks(&self) -> usize {
        self.items
            .iter()
            .filter(|item| {
                matches!(
                    item,
                    PlanItem::ThemeChange { transition, .. } if transition.forces_page_break()
                )
            })
            .count()
    }
}

/// Resolves themes across a document and flattens it into a plan.
///
/// A section's declared theme applies to that section and everything nested
/// inside it, unless a nested section declares its own. Leaving a section
/// restores the theme that was in effect outside it.
///
/// # Errors
///
/// Returns [`DomainError::UnknownTheme`] when a section names a theme the theme
/// set does not define.
pub fn plan_document(document: &Document, themes: &ThemeSet) -> Result<LayoutPlan, DomainError> {
    let mut planner = Planner {
        current: themes.default_theme().clone(),
        pending: None,
        items: Vec::new(),
    };

    if let Some(title) = document.title() {
        planner.push(PlanItem::Title(title.clone()));
    }

    planner.walk(document.body(), themes)?;

    Ok(LayoutPlan {
        initial: themes.default_theme().clone(),
        items: planner.items,
    })
}

/// Builds the plan while tracking which theme is in effect.
///
/// A theme change is recorded as *pending* and only committed when content
/// actually follows it. Without that, leaving the last themed section of a
/// document would emit a change with nothing after it — and, if the geometry
/// differed, a page break producing a blank final page.
struct Planner {
    current: Theme,
    pending: Option<Theme>,
    items: Vec<PlanItem>,
}

impl Planner {
    fn set_theme(&mut self, theme: Theme) {
        self.pending = Some(theme);
    }

    fn push(&mut self, item: PlanItem) {
        if let Some(pending) = self.pending.take() {
            let transition = ThemeTransition::classify(&self.current, &pending);
            if transition.requires_emission() {
                self.items.push(PlanItem::ThemeChange {
                    theme: pending.clone(),
                    transition,
                });
            }
            self.current = pending;
        }
        self.items.push(item);
    }

    fn walk(&mut self, blocks: &[Block], themes: &ThemeSet) -> Result<(), DomainError> {
        for block in blocks {
            self.walk_block(block, themes)?;
        }
        Ok(())
    }

    fn walk_block(&mut self, block: &Block, themes: &ThemeSet) -> Result<(), DomainError> {
        match block {
            Block::Paragraph(paragraph) => {
                self.push(PlanItem::Paragraph(paragraph.text().clone()));
            }
            Block::Section(section) => self.walk_section(section, themes)?,
            Block::Verbatim(verbatim) => self.push(PlanItem::Verbatim(verbatim.clone())),
            Block::Break(kind) => self.push(PlanItem::Break(*kind)),
            Block::Titled { title, block } => {
                self.push(PlanItem::BlockTitle(title.clone()));
                self.walk_block(block, themes)?;
            }
            Block::Admonition(admonition) => {
                let children = self.group(admonition.body(), themes)?;
                self.push(PlanItem::Group {
                    kind: GroupKind::Admonition(admonition.kind()),
                    children,
                });
            }
            Block::Quotation(quotation) => {
                let children = self.group(quotation.body(), themes)?;
                self.push(PlanItem::Group {
                    kind: GroupKind::Quotation {
                        kind: quotation.kind(),
                        attribution: quotation.attribution().cloned(),
                        citation: quotation.citation().cloned(),
                    },
                    children,
                });
            }
            Block::Container(container) => {
                let children = self.group(container.body(), themes)?;
                self.push(PlanItem::Group {
                    kind: GroupKind::Container(container.kind()),
                    children,
                });
            }
            Block::List(list) => {
                let mut items = Vec::new();
                for (index, item) in list.items().iter().enumerate() {
                    let children = self.group(item.body(), themes)?;
                    items.push(PlanItem::Group {
                        kind: GroupKind::ListItem {
                            term: item.term().cloned(),
                            position: index + 1,
                        },
                        children,
                    });
                }
                self.push(PlanItem::Group {
                    kind: GroupKind::List(list.kind()),
                    children: items,
                });
            }
        }
        Ok(())
    }

    /// Plans a group's content into its own sequence.
    ///
    /// A theme change is committed before the group is entered, not inside it:
    /// only a section can change the theme, and a section cannot be nested in
    /// a group, so a group's content is always set under one theme.
    fn group(&mut self, blocks: &[Block], themes: &ThemeSet) -> Result<Vec<PlanItem>, DomainError> {
        let outer = std::mem::take(&mut self.items);
        self.walk(blocks, themes)?;
        Ok(std::mem::replace(&mut self.items, outer))
    }

    fn walk_section(&mut self, section: &Section, themes: &ThemeSet) -> Result<(), DomainError> {
        let outer = self.pending.clone().unwrap_or_else(|| self.current.clone());

        if let Some(id) = section.theme() {
            let theme = themes
                .get(id)
                .ok_or_else(|| DomainError::UnknownTheme {
                    id: id.to_string(),
                    section: section.heading().to_string(),
                })?
                .clone();
            self.set_theme(theme);
        }

        self.push(PlanItem::Heading {
            text: section.heading().clone(),
            level: section.level(),
        });

        self.walk(section.body(), themes)?;

        self.set_theme(outer);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use adocpdf_core::document::Paragraph;
    use adocpdf_core::geometry::{Margins, PageGeometry};
    use adocpdf_core::length::Length;
    use adocpdf_core::theme::{ThemeId, built_in_default_theme};
    use adocpdf_core::typography::{FontFamily, Typography};

    use super::*;

    fn text(value: &str) -> InlineText {
        InlineText::new(value)
    }

    fn paragraph(value: &str) -> Block {
        Block::Paragraph(Paragraph::new(text(value)))
    }

    fn section(heading: &str) -> Section {
        Section::new(text(heading), HeadingLevel::new(1).unwrap())
    }

    fn wide_theme() -> Theme {
        let geometry = PageGeometry::new(
            Length::from_millimeters(420.0).unwrap(),
            Length::from_millimeters(297.0).unwrap(),
            Margins::uniform(Length::from_millimeters(20.0).unwrap()),
        )
        .unwrap();
        Theme::new(geometry, built_in_default_theme().typography().clone())
    }

    fn restyled_theme() -> Theme {
        Theme::new(
            *built_in_default_theme().geometry(),
            Typography::new(
                FontFamily::new("Noto Serif").unwrap(),
                Length::from_points(13.0).unwrap(),
                Length::from_points(18.0).unwrap(),
            ),
        )
    }

    fn themes() -> ThemeSet {
        ThemeSet::default()
            .with(ThemeId::new("wide").unwrap(), wide_theme())
            .with(ThemeId::new("restyled").unwrap(), restyled_theme())
            .with(ThemeId::new("same").unwrap(), built_in_default_theme())
    }

    fn transitions(plan: &LayoutPlan) -> Vec<ThemeTransition> {
        plan.items()
            .iter()
            .filter_map(|item| match item {
                PlanItem::ThemeChange { transition, .. } => Some(*transition),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn an_unthemed_document_uses_the_default_throughout() {
        let document = Document::new()
            .with_title(text("Report"))
            .with_block(paragraph("Body."));

        let plan = plan_document(&document, &themes()).unwrap();

        assert_eq!(plan.initial_theme(), &built_in_default_theme());
        assert!(
            transitions(&plan).is_empty(),
            "nothing overrides the default, so nothing should change"
        );
    }

    #[test]
    fn a_section_theme_applies_to_its_nested_content() {
        let inner = section("Details").with_block(paragraph("Nested body."));
        let outer = section("Overview")
            .with_theme(ThemeId::new("wide").unwrap())
            .with_block(Block::Section(inner));
        let document = Document::new().with_block(Block::Section(outer));

        let plan = plan_document(&document, &themes()).unwrap();

        let PlanItem::ThemeChange { theme, .. } = &plan.items()[0] else {
            panic!("expected the theme change to come before the heading");
        };
        assert_eq!(theme, &wide_theme());
        assert_eq!(
            transitions(&plan).len(),
            1,
            "the nested section inherits, so it must not cause a second change"
        );
    }

    #[test]
    fn a_nested_override_wins_over_its_parent() {
        let inner = section("Details")
            .with_theme(ThemeId::new("restyled").unwrap())
            .with_block(paragraph("Nested body."));
        let outer = section("Overview")
            .with_theme(ThemeId::new("wide").unwrap())
            .with_block(Block::Section(inner));
        let document = Document::new().with_block(Block::Section(outer));

        let plan = plan_document(&document, &themes()).unwrap();
        let changed: Vec<&Theme> = plan
            .items()
            .iter()
            .filter_map(|item| match item {
                PlanItem::ThemeChange { theme, .. } => Some(theme),
                _ => None,
            })
            .collect();

        assert_eq!(changed, vec![&wide_theme(), &restyled_theme()]);
    }

    #[test]
    fn leaving_a_themed_section_restores_the_outer_theme() {
        let themed = section("Aside")
            .with_theme(ThemeId::new("wide").unwrap())
            .with_block(paragraph("Aside body."));
        let document = Document::new()
            .with_block(Block::Section(themed))
            .with_block(paragraph("Back to normal."));

        let plan = plan_document(&document, &themes()).unwrap();

        let last_change = plan
            .items()
            .iter()
            .rev()
            .find_map(|item| match item {
                PlanItem::ThemeChange { theme, .. } => Some(theme),
                _ => None,
            })
            .expect("the outer theme must be restored");

        assert_eq!(last_change, &built_in_default_theme());
    }

    #[test]
    fn a_themed_section_at_the_end_does_not_emit_a_trailing_change() {
        let themed = section("Appendix")
            .with_theme(ThemeId::new("wide").unwrap())
            .with_block(paragraph("Last words."));
        let document = Document::new().with_block(Block::Section(themed));

        let plan = plan_document(&document, &themes()).unwrap();

        assert_eq!(
            transitions(&plan).len(),
            1,
            "restoring a theme with no content after it would leave a blank page"
        );
    }

    #[test]
    fn a_geometry_change_is_planned_as_a_page_break() {
        let themed = section("Wide")
            .with_theme(ThemeId::new("wide").unwrap())
            .with_block(paragraph("Wide body."));
        let document = Document::new().with_block(Block::Section(themed));

        let plan = plan_document(&document, &themes()).unwrap();

        assert_eq!(transitions(&plan), vec![ThemeTransition::PageGeometry]);
        assert_eq!(plan.forced_page_breaks(), 1);
    }

    #[test]
    fn a_typography_change_is_planned_without_a_page_break() {
        let themed = section("Restyled")
            .with_theme(ThemeId::new("restyled").unwrap())
            .with_block(paragraph("Restyled body."));
        let document = Document::new().with_block(Block::Section(themed));

        let plan = plan_document(&document, &themes()).unwrap();

        assert_eq!(transitions(&plan), vec![ThemeTransition::TypographyOnly]);
        assert_eq!(plan.forced_page_breaks(), 0);
    }

    #[test]
    fn declaring_the_theme_already_in_effect_changes_nothing() {
        let themed = section("Same")
            .with_theme(ThemeId::new("same").unwrap())
            .with_block(paragraph("Body."));
        let plain = section("Plain").with_block(paragraph("Body."));

        let with_redundant = plan_document(
            &Document::new().with_block(Block::Section(themed)),
            &themes(),
        )
        .unwrap();
        let without = plan_document(
            &Document::new().with_block(Block::Section(plain)),
            &themes(),
        )
        .unwrap();

        assert_eq!(
            with_redundant.items().len(),
            without.items().len(),
            "a redundant declaration must not reach the output at all"
        );
        assert!(transitions(&with_redundant).is_empty());
    }

    #[test]
    fn a_section_naming_an_undefined_theme_is_rejected() {
        let themed = section("Appendix").with_theme(ThemeId::new("absent").unwrap());
        let document = Document::new().with_block(Block::Section(themed));

        let error = plan_document(&document, &themes()).unwrap_err();

        assert_eq!(
            error,
            DomainError::UnknownTheme {
                id: "absent".to_owned(),
                section: "Appendix".to_owned(),
            }
        );
    }

    #[test]
    fn the_plan_keeps_the_document_in_order() {
        let document = Document::new()
            .with_title(text("Report"))
            .with_block(paragraph("Intro."))
            .with_block(Block::Section(
                section("Body").with_block(paragraph("Detail.")),
            ));

        let plan = plan_document(&document, &themes()).unwrap();

        assert_eq!(
            plan.items(),
            &[
                PlanItem::Title(text("Report")),
                PlanItem::Paragraph(text("Intro.")),
                PlanItem::Heading {
                    text: text("Body"),
                    level: HeadingLevel::new(1).unwrap(),
                },
                PlanItem::Paragraph(text("Detail.")),
            ]
        );
    }
}
