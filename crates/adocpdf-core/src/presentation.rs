//! How a block is set, when the source asks for something other than the
//! default.
//!
//! Every type here is a value object: it is built from what the source
//! declared, it validates then, and it is immutable afterwards. That is the
//! point. The alternative — carrying the attribute list's strings inward and
//! matching on them wherever they are used — would mean the same string is
//! interpreted in several places, with nothing to keep those places agreeing,
//! and would put unvalidated document text on the far side of the injection
//! boundary.
//!
//! The vocabulary lives here, in the model, rather than in the adapter that
//! reads it. A role means the same thing on a paragraph as on a list, and one
//! definition is how that stays true.

use std::error::Error;
use std::fmt;

/// The role that marks a paragraph as the document's opening passage.
pub const LEAD_ROLE: &str = "lead";

/// How a paragraph's lines sit on the measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// Flush left, ragged right.
    Left,
    /// Centred, ragged on both sides.
    Center,
    /// Flush right, ragged left.
    Right,
    /// Flush on both sides, spaced to fit.
    Justify,
}

impl Alignment {
    /// The alignment the role `name` asks for, if it names one.
    #[must_use]
    pub fn from_role(name: &str) -> Option<Self> {
        match name {
            "text-left" => Some(Self::Left),
            "text-center" => Some(Self::Center),
            "text-right" => Some(Self::Right),
            "text-justify" => Some(Self::Justify),
            _ => None,
        }
    }
}

/// What a paragraph's attribute list asked for.
///
/// The default is *nothing asked for*, which is different from asking for the
/// theme's own settings: a paragraph that declares no alignment must be set
/// however the theme sets body text, and must go on being set that way when
/// the theme changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ParagraphPresentation {
    alignment: Option<Alignment>,
    lead: bool,
}

impl ParagraphPresentation {
    /// A paragraph that asked for nothing.
    #[must_use]
    pub const fn body() -> Self {
        Self {
            alignment: None,
            lead: false,
        }
    }

    /// Declares how the paragraph's lines sit on the measure.
    #[must_use]
    pub const fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    /// Marks the paragraph as a lead paragraph.
    #[must_use]
    pub const fn as_lead(mut self) -> Self {
        self.lead = true;
        self
    }

    /// The alignment the paragraph declared, if it declared one.
    #[must_use]
    pub const fn alignment(self) -> Option<Alignment> {
        self.alignment
    }

    /// Whether the paragraph is a lead paragraph.
    #[must_use]
    pub const fn is_lead(self) -> bool {
        self.lead
    }

    /// Whether the paragraph asked for nothing at all.
    #[must_use]
    pub const fn is_body(self) -> bool {
        self.alignment.is_none() && !self.lead
    }
}

/// The shape an unordered list's marker is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListMarker {
    /// A filled round bullet.
    Disc,
    /// A hollow round bullet.
    Circle,
    /// A filled square bullet.
    Square,
}

impl ListMarker {
    /// The marker the block style `name` asks for, if it names one.
    #[must_use]
    pub fn from_style(name: &str) -> Option<Self> {
        match name {
            "disc" => Some(Self::Disc),
            "circle" => Some(Self::Circle),
            "square" => Some(Self::Square),
            _ => None,
        }
    }

    /// The character a reader sees.
    #[must_use]
    pub const fn glyph(self) -> char {
        match self {
            Self::Disc => '\u{2022}',
            Self::Circle => '\u{25e6}',
            Self::Square => '\u{25aa}',
        }
    }
}

/// The number an ordered list counts from.
///
/// Held as a number rather than as the string the source wrote, so that the
/// only place a start can be malformed is the one place it is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListStart(usize);

impl ListStart {
    /// Where an ordered list counts from when it does not say.
    pub const FIRST: Self = Self(1);

    /// Reads the start an attribute list declared.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidListStart`] when `value` is not a non-negative whole
    /// number. A list cannot count from `two` or from `-1`, and accepting
    /// either would mean deciding much later what such a list numbers from.
    pub fn parse(value: &str) -> Result<Self, InvalidListStart> {
        value
            .trim()
            .parse::<usize>()
            .map(Self)
            .map_err(|_| InvalidListStart {
                value: value.to_owned(),
            })
    }

    /// The number as written.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }

    /// The number the item at `index` carries, counting from zero.
    ///
    /// Saturating, so that a start near the top of the range produces a list
    /// that numbers oddly rather than one that panics or wraps to zero.
    #[must_use]
    pub const fn position_of(self, index: usize) -> usize {
        self.0.saturating_add(index)
    }
}

impl Default for ListStart {
    fn default() -> Self {
        Self::FIRST
    }
}

/// A declared start that is not a number a list could count from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidListStart {
    /// What the source declared.
    pub value: String,
}

impl fmt::Display for InvalidListStart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a list's start must be a non-negative whole number, got {:?}",
            self.value
        )
    }
}

impl Error for InvalidListStart {}

/// How a list is presented, beyond the kind of list it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ListForm {
    /// Items one under another, each beginning beside its marker.
    #[default]
    Stacked,
    /// Terms in a column of their own, each description beside its term.
    Horizontal,
    /// Each term set as a question, with its answer beneath.
    QuestionsAndAnswers,
    /// Items whose markers show whether each is done.
    Checklist,
}

impl ListForm {
    /// The form the block style `name` asks for, if it names one.
    ///
    /// A checklist is not here: it is not declared by an attribute at all, but
    /// read from the items themselves.
    #[must_use]
    pub fn from_style(name: &str) -> Option<Self> {
        match name {
            "horizontal" => Some(Self::Horizontal),
            "qanda" => Some(Self::QuestionsAndAnswers),
            _ => None,
        }
    }
}

/// What a list's attribute list asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ListPresentation {
    marker: Option<ListMarker>,
    start: ListStart,
    form: ListForm,
}

impl ListPresentation {
    /// A list that asked for nothing.
    #[must_use]
    pub fn stacked() -> Self {
        Self::default()
    }

    /// Declares the shape of the list's markers.
    #[must_use]
    pub const fn with_marker(mut self, marker: ListMarker) -> Self {
        self.marker = Some(marker);
        self
    }

    /// Declares the number the list counts from.
    #[must_use]
    pub const fn with_start(mut self, start: ListStart) -> Self {
        self.start = start;
        self
    }

    /// Declares how the list is set.
    #[must_use]
    pub const fn with_form(mut self, form: ListForm) -> Self {
        self.form = form;
        self
    }

    /// The marker shape the list declared, if it declared one.
    #[must_use]
    pub const fn marker(self) -> Option<ListMarker> {
        self.marker
    }

    /// The number the list counts from.
    #[must_use]
    pub const fn start(self) -> ListStart {
        self.start
    }

    /// How the list is set.
    #[must_use]
    pub const fn form(self) -> ListForm {
        self.form
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_alignment_roles_are_the_four_the_language_names() {
        assert_eq!(Alignment::from_role("text-left"), Some(Alignment::Left));
        assert_eq!(Alignment::from_role("text-center"), Some(Alignment::Center));
        assert_eq!(Alignment::from_role("text-right"), Some(Alignment::Right));
        assert_eq!(
            Alignment::from_role("text-justify"),
            Some(Alignment::Justify)
        );
    }

    #[test]
    fn a_role_that_names_no_alignment_yields_none() {
        assert_eq!(Alignment::from_role("centre"), None);
        assert_eq!(Alignment::from_role("lead"), None);
    }

    #[test]
    fn a_paragraph_that_declares_nothing_is_body_text() {
        let presentation = ParagraphPresentation::body();

        assert!(presentation.is_body());
        assert_eq!(presentation.alignment(), None);
        assert!(!presentation.is_lead());
    }

    #[test]
    fn a_paragraph_remembers_what_it_declared() {
        let presentation = ParagraphPresentation::body()
            .with_alignment(Alignment::Center)
            .as_lead();

        assert_eq!(presentation.alignment(), Some(Alignment::Center));
        assert!(presentation.is_lead());
        assert!(!presentation.is_body());
    }

    #[test]
    fn each_marker_shape_has_a_glyph_of_its_own() {
        let glyphs = [
            ListMarker::Disc.glyph(),
            ListMarker::Circle.glyph(),
            ListMarker::Square.glyph(),
        ];

        assert_eq!(
            glyphs
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            glyphs.len(),
            "two shapes drawn the same are one shape"
        );
    }

    #[test]
    fn the_marker_styles_are_the_three_the_language_names() {
        assert_eq!(ListMarker::from_style("disc"), Some(ListMarker::Disc));
        assert_eq!(ListMarker::from_style("circle"), Some(ListMarker::Circle));
        assert_eq!(ListMarker::from_style("square"), Some(ListMarker::Square));
        assert_eq!(ListMarker::from_style("triangle"), None);
    }

    #[test]
    fn a_list_counts_from_one_unless_it_says_otherwise() {
        assert_eq!(ListStart::default(), ListStart::FIRST);
        assert_eq!(ListStart::FIRST.position_of(0), 1);
        assert_eq!(ListStart::FIRST.position_of(2), 3);
    }

    #[test]
    fn a_declared_start_is_the_number_the_first_item_carries() {
        let start = ListStart::parse("4").unwrap();

        assert_eq!(start.get(), 4);
        assert_eq!(start.position_of(0), 4);
        assert_eq!(start.position_of(1), 5);
    }

    #[test]
    fn a_start_that_is_not_a_number_is_rejected() {
        let error = ListStart::parse("two").unwrap_err();

        assert!(
            error.to_string().contains("two"),
            "the message must report what was declared, got: {error}"
        );
    }

    #[test]
    fn a_negative_start_is_rejected() {
        assert!(ListStart::parse("-1").is_err());
    }

    #[test]
    fn a_start_at_the_top_of_the_range_saturates_rather_than_wrapping() {
        let start = ListStart::parse(&usize::MAX.to_string()).unwrap();

        assert_eq!(start.position_of(1), usize::MAX);
    }

    #[test]
    fn the_list_forms_are_the_two_an_attribute_can_declare() {
        assert_eq!(
            ListForm::from_style("horizontal"),
            Some(ListForm::Horizontal)
        );
        assert_eq!(
            ListForm::from_style("qanda"),
            Some(ListForm::QuestionsAndAnswers)
        );
        assert_eq!(
            ListForm::from_style("checklist"),
            None,
            "a checklist is read from the items, not from an attribute"
        );
    }

    #[test]
    fn a_list_that_declares_nothing_is_stacked_and_counts_from_one() {
        let presentation = ListPresentation::stacked();

        assert_eq!(presentation.marker(), None);
        assert_eq!(presentation.start(), ListStart::FIRST);
        assert_eq!(presentation.form(), ListForm::Stacked);
    }

    #[test]
    fn a_list_remembers_what_it_declared() {
        let presentation = ListPresentation::stacked()
            .with_marker(ListMarker::Square)
            .with_start(ListStart::parse("7").unwrap())
            .with_form(ListForm::Horizontal);

        assert_eq!(presentation.marker(), Some(ListMarker::Square));
        assert_eq!(presentation.start().get(), 7);
        assert_eq!(presentation.form(), ListForm::Horizontal);
    }
}
