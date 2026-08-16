//! Themes, and the set of them a document draws on.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::geometry::{Margins, PageGeometry};
use crate::length::Length;
use crate::typography::{FontFamily, Typography};

/// The name a document uses to refer to a theme.
///
/// Constrained to lowercase letters, digits and hyphens so an identifier is
/// safe to place in a diagnostic or a file name without further quoting.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThemeId(String);

impl ThemeId {
    /// The longest identifier accepted.
    pub const MAX_LENGTH: usize = 64;

    /// Creates a theme identifier.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidThemeId`] when the identifier is empty, longer than
    /// [`ThemeId::MAX_LENGTH`], or contains anything other than a lowercase
    /// ASCII letter, a digit, or a hyphen.
    pub fn new(id: &str) -> Result<Self, InvalidThemeId> {
        if id.is_empty() {
            return Err(InvalidThemeId::Empty);
        }
        if id.chars().count() > Self::MAX_LENGTH {
            return Err(InvalidThemeId::TooLong {
                length: id.chars().count(),
            });
        }
        if let Some(character) = id
            .chars()
            .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-'))
        {
            return Err(InvalidThemeId::ForbiddenCharacter { character });
        }
        Ok(Self(id.to_owned()))
    }

    /// The identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ThemeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a proposed theme identifier is not usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidThemeId {
    /// The identifier was empty.
    Empty,
    /// The identifier exceeded [`ThemeId::MAX_LENGTH`].
    TooLong {
        /// How many characters the identifier had.
        length: usize,
    },
    /// The identifier contained a character outside the permitted alphabet.
    ForbiddenCharacter {
        /// The first offending character.
        character: char,
    },
}

impl fmt::Display for InvalidThemeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("theme id must not be empty"),
            Self::TooLong { length } => write!(
                f,
                "theme id must be at most {} characters, got {length}",
                ThemeId::MAX_LENGTH
            ),
            Self::ForbiddenCharacter { character } => write!(
                f,
                "theme id must contain only lowercase letters, digits and hyphens, but \
                 contains {character:?}"
            ),
        }
    }
}

impl Error for InvalidThemeId {}

/// A complete visual treatment: how the page is shaped and how text is set.
///
/// The two halves are kept separately addressable because changing one has a
/// consequence changing the other does not: a page-geometry change forces a
/// page break where a typography change does not.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    geometry: PageGeometry,
    typography: Typography,
}

impl Theme {
    /// Creates a theme from a page geometry and a typography setting.
    #[must_use]
    pub fn new(geometry: PageGeometry, typography: Typography) -> Self {
        Self {
            geometry,
            typography,
        }
    }

    /// How the page is shaped.
    #[must_use]
    pub fn geometry(&self) -> &PageGeometry {
        &self.geometry
    }

    /// How text is set.
    #[must_use]
    pub fn typography(&self) -> &Typography {
        &self.typography
    }
}

/// What changes when the document moves from one theme to another.
///
/// The distinction exists because the two halves of a theme have different
/// consequences: the layout engine starts a new page when page geometry changes
/// mid-document, but continues on the current page when only typography does.
/// Classifying the transition here — rather than discovering it from the
/// engine's behaviour — means the rule is testable without rendering anything,
/// and can be reported to an author before a render happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeTransition {
    /// The two themes are identical. Nothing needs to be emitted at all.
    Unchanged,
    /// Only the typography differs. Text continues on the current page.
    TypographyOnly,
    /// The page geometry differs. The following content starts on a new page.
    PageGeometry,
}

impl ThemeTransition {
    /// Classifies the move from `previous` to `next`.
    #[must_use]
    pub fn classify(previous: &Theme, next: &Theme) -> Self {
        if previous.geometry() != next.geometry() {
            Self::PageGeometry
        } else if previous.typography() != next.typography() {
            Self::TypographyOnly
        } else {
            Self::Unchanged
        }
    }

    /// Whether this transition makes the following content start on a new page.
    #[must_use]
    pub fn forces_page_break(self) -> bool {
        matches!(self, Self::PageGeometry)
    }

    /// Whether this transition requires emitting anything at all.
    #[must_use]
    pub fn requires_emission(self) -> bool {
        !matches!(self, Self::Unchanged)
    }
}

impl fmt::Display for ThemeTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unchanged => f.write_str("no change"),
            Self::TypographyOnly => f.write_str("typography only, continues on the same page"),
            Self::PageGeometry => f.write_str("page geometry, starts a new page"),
        }
    }
}

/// The themes a document may refer to, and the one it falls back on.
///
/// Backed by an ordered map: iteration order is a function of the identifiers
/// alone, never of insertion order or hashing, so a document renders the same
/// way on every run.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeSet {
    default: Theme,
    named: BTreeMap<ThemeId, Theme>,
}

impl ThemeSet {
    /// Creates a set with the given default and no named themes.
    #[must_use]
    pub fn new(default: Theme) -> Self {
        Self {
            default,
            named: BTreeMap::new(),
        }
    }

    /// Adds a named theme, replacing any theme already under that identifier.
    #[must_use]
    pub fn with(mut self, id: ThemeId, theme: Theme) -> Self {
        self.named.insert(id, theme);
        self
    }

    /// The theme applied when nothing overrides it.
    #[must_use]
    pub fn default_theme(&self) -> &Theme {
        &self.default
    }

    /// Looks a theme up by identifier.
    ///
    /// Returns `None` when no theme carries that identifier; the caller decides
    /// whether that is an error, because the answer differs between a document
    /// referring to a missing theme and a caller merely asking.
    #[must_use]
    pub fn get(&self, id: &ThemeId) -> Option<&Theme> {
        self.named.get(id)
    }

    /// The identifiers of every named theme, in a stable order.
    pub fn ids(&self) -> impl Iterator<Item = &ThemeId> {
        self.named.keys()
    }
}

impl Default for ThemeSet {
    fn default() -> Self {
        Self::new(built_in_default_theme())
    }
}

/// The theme applied to a document that specifies none.
///
/// A4 with 20 mm margins, set in the default body face at 11 pt on 15.4 pt
/// leading. The figures are chosen to produce a readable page with no
/// configuration, which is what the theming specification requires of the
/// default.
///
/// # Panics
///
/// Does not panic. The values are constants known to satisfy every validation
/// rule, and the test below proves it, but the constructors are fallible so the
/// unwraps are written out rather than hidden.
#[must_use]
#[allow(
    clippy::expect_used,
    reason = "the arguments are compile-time constants, and \
              `the_built_in_default_is_valid_and_a4_sized` proves they satisfy every rule. \
              Making this fallible would push an error that cannot occur onto every caller."
)]
pub fn built_in_default_theme() -> Theme {
    let geometry = PageGeometry::new(
        Length::from_millimeters(210.0).expect("A4 width is a valid length"),
        Length::from_millimeters(297.0).expect("A4 height is a valid length"),
        Margins::uniform(Length::from_millimeters(20.0).expect("20mm is a valid length")),
    )
    .expect("A4 with 20mm margins leaves a printable area");

    let typography = Typography::new(
        FontFamily::new(DEFAULT_FONT_FAMILY).expect("the default family name is well formed"),
        Length::from_points(11.0).expect("11pt is a valid length"),
        Length::from_points(15.4).expect("15.4pt is a valid length"),
    );

    Theme::new(geometry, typography)
}

/// The family name of the body face.
///
/// This must name the font the renderer actually embeds; the two are reconciled
/// when the font is vendored.
pub const DEFAULT_FONT_FAMILY: &str = "DejaVu Sans";

#[cfg(test)]
mod tests {
    use super::*;

    fn theme_with_width(millimeters: f64) -> Theme {
        let geometry = PageGeometry::new(
            Length::from_millimeters(millimeters).unwrap(),
            Length::from_millimeters(297.0).unwrap(),
            Margins::uniform(Length::from_millimeters(10.0).unwrap()),
        )
        .unwrap();
        Theme::new(geometry, built_in_default_theme().typography().clone())
    }

    #[test]
    fn the_built_in_default_is_valid_and_a4_sized() {
        let theme = built_in_default_theme();

        assert!((theme.geometry().width().millimeters() - 210.0).abs() < 1e-9);
        assert!((theme.geometry().height().millimeters() - 297.0).abs() < 1e-9);
        assert!((theme.typography().size().points() - 11.0).abs() < 1e-9);
    }

    #[test]
    fn a_new_set_falls_back_on_the_built_in_default() {
        let set = ThemeSet::default();

        assert_eq!(set.default_theme(), &built_in_default_theme());
    }

    #[test]
    fn a_named_theme_can_be_looked_up() {
        let id = ThemeId::new("wide").unwrap();
        let set = ThemeSet::default().with(id.clone(), theme_with_width(420.0));

        let found = set.get(&id).expect("the theme was just added");

        assert!((found.geometry().width().millimeters() - 420.0).abs() < 1e-9);
    }

    #[test]
    fn looking_up_an_unknown_theme_finds_nothing() {
        let set = ThemeSet::default();

        assert!(set.get(&ThemeId::new("absent").unwrap()).is_none());
    }

    #[test]
    fn adding_a_theme_twice_keeps_the_later_one() {
        let id = ThemeId::new("body").unwrap();

        let set = ThemeSet::default()
            .with(id.clone(), theme_with_width(300.0))
            .with(id.clone(), theme_with_width(400.0));

        let found = set.get(&id).unwrap();
        assert!((found.geometry().width().millimeters() - 400.0).abs() < 1e-9);
    }

    #[test]
    fn identifiers_are_listed_in_a_stable_order() {
        let set = ThemeSet::default()
            .with(ThemeId::new("zebra").unwrap(), theme_with_width(300.0))
            .with(ThemeId::new("alpha").unwrap(), theme_with_width(300.0))
            .with(ThemeId::new("middle").unwrap(), theme_with_width(300.0));

        let ids: Vec<&str> = set.ids().map(ThemeId::as_str).collect();

        assert_eq!(
            ids,
            ["alpha", "middle", "zebra"],
            "iteration order must depend on the identifiers alone, so renders are reproducible"
        );
    }

    #[test]
    fn changing_the_page_size_forces_a_page_break() {
        let transition =
            ThemeTransition::classify(&theme_with_width(210.0), &theme_with_width(420.0));

        assert_eq!(transition, ThemeTransition::PageGeometry);
        assert!(transition.forces_page_break());
    }

    #[test]
    fn changing_only_the_typography_does_not_force_a_page_break() {
        let base = theme_with_width(210.0);
        let restyled = Theme::new(
            *base.geometry(),
            Typography::new(
                FontFamily::new("Noto Serif").unwrap(),
                Length::from_points(13.0).unwrap(),
                Length::from_points(18.0).unwrap(),
            ),
        );

        let transition = ThemeTransition::classify(&base, &restyled);

        assert_eq!(transition, ThemeTransition::TypographyOnly);
        assert!(!transition.forces_page_break());
        assert!(
            transition.requires_emission(),
            "the new typography still has to reach the output"
        );
    }

    #[test]
    fn a_transition_between_identical_themes_changes_nothing() {
        let transition =
            ThemeTransition::classify(&theme_with_width(210.0), &theme_with_width(210.0));

        assert_eq!(transition, ThemeTransition::Unchanged);
        assert!(!transition.forces_page_break());
        assert!(
            !transition.requires_emission(),
            "a redundant declaration must leave the output byte-identical"
        );
    }

    #[test]
    fn a_geometry_change_outranks_a_simultaneous_typography_change() {
        let base = theme_with_width(210.0);
        let both = Theme::new(
            *theme_with_width(420.0).geometry(),
            Typography::new(
                FontFamily::new("Noto Serif").unwrap(),
                Length::from_points(13.0).unwrap(),
                Length::from_points(18.0).unwrap(),
            ),
        );

        let transition = ThemeTransition::classify(&base, &both);

        assert_eq!(
            transition,
            ThemeTransition::PageGeometry,
            "when both halves change, the page break still has to happen"
        );
    }

    #[test]
    fn a_transition_describes_its_own_consequence() {
        assert!(
            ThemeTransition::PageGeometry
                .to_string()
                .contains("new page")
        );
        assert!(
            ThemeTransition::TypographyOnly
                .to_string()
                .contains("same page")
        );
    }

    #[test]
    fn a_well_formed_identifier_is_accepted() {
        assert_eq!(ThemeId::new("chapter-2").unwrap().as_str(), "chapter-2");
    }

    #[test]
    fn an_empty_identifier_is_rejected() {
        assert_eq!(ThemeId::new(""), Err(InvalidThemeId::Empty));
    }

    #[test]
    fn an_identifier_with_uppercase_or_markup_is_rejected() {
        for id in ["Chapter", "chapter 2", "chapter#", "chapter/../etc"] {
            assert!(
                matches!(
                    ThemeId::new(id),
                    Err(InvalidThemeId::ForbiddenCharacter { .. })
                ),
                "{id:?} must not be accepted as a theme id"
            );
        }
    }

    #[test]
    fn an_overlong_identifier_is_rejected() {
        let id = "a".repeat(ThemeId::MAX_LENGTH + 1);

        assert!(matches!(
            ThemeId::new(&id),
            Err(InvalidThemeId::TooLong { .. })
        ));
    }
}
