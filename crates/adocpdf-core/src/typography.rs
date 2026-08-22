//! How text is set: which face, at what size, with how much leading.

use std::error::Error;
use std::fmt;

use crate::length::Length;

/// The name of a font family.
///
/// A family name reaches the output as part of a rendering instruction rather
/// than as content, so it cannot go through the content escaper. It is
/// constrained here to a closed alphabet instead — letters, digits, spaces and
/// hyphens — which keeps the set of characters that can appear in a structural
/// position finite and known. See `design.md`, decision D2.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontFamily(String);

impl FontFamily {
    /// The longest family name accepted.
    ///
    /// Real family names are far shorter; the bound exists so an absurd value
    /// cannot reach the output at all.
    pub const MAX_LENGTH: usize = 64;

    /// Creates a font family name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFontFamily`] when the name is empty, longer than
    /// [`FontFamily::MAX_LENGTH`], or contains a character outside the
    /// permitted alphabet.
    pub fn new(name: &str) -> Result<Self, InvalidFontFamily> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(InvalidFontFamily::Empty);
        }
        if trimmed.chars().count() > Self::MAX_LENGTH {
            return Err(InvalidFontFamily::TooLong {
                length: trimmed.chars().count(),
            });
        }
        if let Some(character) = trimmed
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == ' ' || *c == '-'))
        {
            return Err(InvalidFontFamily::ForbiddenCharacter { character });
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The family name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FontFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a proposed font family name is not usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidFontFamily {
    /// The name was empty, or contained only whitespace.
    Empty,
    /// The name exceeded [`FontFamily::MAX_LENGTH`].
    TooLong {
        /// How many characters the name had.
        length: usize,
    },
    /// The name contained a character outside the permitted alphabet.
    ForbiddenCharacter {
        /// The first offending character.
        character: char,
    },
}

impl fmt::Display for InvalidFontFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("font family must not be empty"),
            Self::TooLong { length } => write!(
                f,
                "font family must be at most {} characters, got {length}",
                FontFamily::MAX_LENGTH
            ),
            Self::ForbiddenCharacter { character } => write!(
                f,
                "font family must contain only letters, digits, spaces and hyphens, but \
                 contains {character:?}"
            ),
        }
    }
}

impl Error for InvalidFontFamily {}

/// A language tag, naming the language text is written in.
///
/// The tag reaches the output as part of a rendering instruction, so like a
/// family name it is held to a closed alphabet rather than passed through the
/// content escaper.
///
/// It matters for more than correctness of metadata: hyphenation is applied
/// only when the language is known, so a theme that names none gets no
/// hyphenation however it is otherwise set.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageTag(String);

impl LanguageTag {
    /// The longest tag accepted.
    pub const MAX_LENGTH: usize = 12;

    /// Creates a language tag.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLanguageTag`] when the tag is empty, longer than
    /// [`LanguageTag::MAX_LENGTH`], or contains anything but ASCII letters and
    /// hyphens.
    pub fn new(tag: &str) -> Result<Self, InvalidLanguageTag> {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            return Err(InvalidLanguageTag::Empty);
        }
        if trimmed.chars().count() > Self::MAX_LENGTH {
            return Err(InvalidLanguageTag::TooLong {
                length: trimmed.chars().count(),
            });
        }
        if let Some(character) = trimmed
            .chars()
            .find(|c| !(c.is_ascii_alphabetic() || *c == '-'))
        {
            return Err(InvalidLanguageTag::ForbiddenCharacter { character });
        }
        Ok(Self(trimmed.to_ascii_lowercase()))
    }

    /// The tag.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LanguageTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a proposed language tag is not usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidLanguageTag {
    /// The tag was empty, or contained only whitespace.
    Empty,
    /// The tag exceeded [`LanguageTag::MAX_LENGTH`].
    TooLong {
        /// How many characters the tag had.
        length: usize,
    },
    /// The tag contained a character outside the permitted alphabet.
    ForbiddenCharacter {
        /// The first offending character.
        character: char,
    },
}

impl fmt::Display for InvalidLanguageTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("language tag must not be empty"),
            Self::TooLong { length } => write!(
                f,
                "language tag must be at most {} characters, got {length}",
                LanguageTag::MAX_LENGTH
            ),
            Self::ForbiddenCharacter { character } => write!(
                f,
                "language tag must contain only letters and hyphens, but contains {character:?}"
            ),
        }
    }
}

impl Error for InvalidLanguageTag {}

/// How strongly the layout should avoid stranding a single line.
///
/// A percentage, where zero means "do not bother" and larger values weigh the
/// avoidance more heavily against other considerations. It is not a boolean
/// because the engine treats it as a cost traded off against everything else,
/// and flattening that to on/off would misrepresent what the setting does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Avoidance(u32);

impl Avoidance {
    /// The largest value accepted.
    pub const MAX_PERCENT: u32 = 1000;

    /// The default: avoid stranding a line, weighed normally.
    pub const DEFAULT: Self = Self(100);

    /// Creates an avoidance weight from a percentage.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidAvoidance`] when the percentage exceeds
    /// [`Avoidance::MAX_PERCENT`].
    pub const fn from_percent(percent: u32) -> Result<Self, InvalidAvoidance> {
        if percent > Self::MAX_PERCENT {
            return Err(InvalidAvoidance { percent });
        }
        Ok(Self(percent))
    }

    /// The weight as a percentage.
    #[must_use]
    pub const fn percent(self) -> u32 {
        self.0
    }
}

impl Default for Avoidance {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A proposed avoidance weight outside the permitted range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidAvoidance {
    /// The percentage that was proposed.
    pub percent: u32,
}

impl fmt::Display for InvalidAvoidance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "widow and orphan avoidance must be at most {}%, got {}%",
            Avoidance::MAX_PERCENT,
            self.percent
        )
    }
}

impl Error for InvalidAvoidance {}

/// The family name monospaced content is set in when a theme names none.
///
/// This must name a family the renderer actually embeds; the two are reconciled
/// by a test in the infrastructure layer, because that is where the faces live.
pub const DEFAULT_MONOSPACE_FAMILY: &str = "DejaVu Sans Mono";

/// The default monospace family, as a value object.
///
/// # Panics
///
/// Does not panic: the constant is a well-formed family name and the test
/// below proves it.
#[must_use]
#[allow(
    clippy::expect_used,
    reason = "the argument is a compile-time constant proven well formed by \
              `the_default_monospace_family_is_well_formed`"
)]
fn default_monospace_family() -> FontFamily {
    FontFamily::new(DEFAULT_MONOSPACE_FAMILY).expect("the default monospace family is well formed")
}

/// How text is set.
///
/// Leading is the distance between consecutive baselines. It is a length rather
/// than a multiple of the font size so that the value written to the output is
/// the value stated here, with no arithmetic in between to disagree about.
#[derive(Debug, Clone, PartialEq)]
pub struct Typography {
    family: FontFamily,
    monospace_family: FontFamily,
    size: Length,
    leading: Length,
    justified: bool,
    language: Option<LanguageTag>,
    avoidance: Avoidance,
}

impl Typography {
    /// Creates a typography setting.
    ///
    /// Every component is already validated by its own type, so this cannot
    /// fail: a non-positive size or an ill-formed family name is unrepresentable
    /// before it reaches here.
    #[must_use]
    pub fn new(family: FontFamily, size: Length, leading: Length) -> Self {
        Self {
            family,
            monospace_family: default_monospace_family(),
            size,
            leading,
            // Ragged right by default. Optimal line breaking is applied either
            // way — the two are separate decisions, and coupling them would
            // impose a justified measure on every document as a side effect of
            // wanting good breaks.
            justified: false,
            language: None,
            avoidance: Avoidance::DEFAULT,
        }
    }

    /// Sets text flush to both margins.
    #[must_use]
    pub fn justified(mut self, justified: bool) -> Self {
        self.justified = justified;
        self
    }

    /// Names the language the text is written in.
    ///
    /// Hyphenation depends on this: it is applied only when the language is
    /// known, so a theme naming none gets none.
    #[must_use]
    pub fn with_language(mut self, language: LanguageTag) -> Self {
        self.language = Some(language);
        self
    }

    /// Sets how strongly to avoid stranding a single line.
    #[must_use]
    pub fn with_avoidance(mut self, avoidance: Avoidance) -> Self {
        self.avoidance = avoidance;
        self
    }

    /// Sets the family verbatim and monospaced content is set in.
    ///
    /// Separate from [`Typography::new`] because a theme that says nothing
    /// about monospace still needs a face for it: verbatim content set in the
    /// body face is not verbatim content, it is body text that happens to have
    /// been typed inside a listing block.
    #[must_use]
    pub fn with_monospace_family(mut self, family: FontFamily) -> Self {
        self.monospace_family = family;
        self
    }

    /// The font family.
    #[must_use]
    pub fn family(&self) -> &FontFamily {
        &self.family
    }

    /// The family verbatim and monospaced content is set in.
    #[must_use]
    pub fn monospace_family(&self) -> &FontFamily {
        &self.monospace_family
    }

    /// The font size.
    #[must_use]
    pub fn size(&self) -> Length {
        self.size
    }

    /// The distance between consecutive baselines.
    #[must_use]
    pub fn leading(&self) -> Length {
        self.leading
    }

    /// Whether text is set flush to both margins.
    #[must_use]
    pub const fn is_justified(&self) -> bool {
        self.justified
    }

    /// The language the text is written in, if the theme names one.
    #[must_use]
    pub fn language(&self) -> Option<&LanguageTag> {
        self.language.as_ref()
    }

    /// How strongly to avoid stranding a single line.
    #[must_use]
    pub const fn avoidance(&self) -> Avoidance {
        self.avoidance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(value: f64) -> Length {
        Length::from_points(value).unwrap()
    }

    #[test]
    fn the_default_monospace_family_is_well_formed() {
        assert!(FontFamily::new(DEFAULT_MONOSPACE_FAMILY).is_ok());
    }

    #[test]
    fn typography_has_a_monospace_family_without_being_asked() {
        let typography = Typography::new(
            FontFamily::new("DejaVu Sans").unwrap(),
            points(11.0),
            points(15.4),
        );

        assert_eq!(
            typography.monospace_family().as_str(),
            DEFAULT_MONOSPACE_FAMILY,
            "verbatim content set in the body face is not verbatim content"
        );
    }

    #[test]
    fn a_theme_can_name_its_own_monospace_family() {
        let typography = Typography::new(
            FontFamily::new("DejaVu Sans").unwrap(),
            points(11.0),
            points(15.4),
        )
        .with_monospace_family(FontFamily::new("Fira Code").unwrap());

        assert_eq!(typography.monospace_family().as_str(), "Fira Code");
    }

    #[test]
    fn an_ordinary_family_name_is_accepted() {
        let family = FontFamily::new("DejaVu Sans").unwrap();

        assert_eq!(family.as_str(), "DejaVu Sans");
    }

    #[test]
    fn a_hyphenated_family_name_is_accepted() {
        assert!(FontFamily::new("Noto-Serif").is_ok());
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        let family = FontFamily::new("  Libertinus Serif  ").unwrap();

        assert_eq!(family.as_str(), "Libertinus Serif");
    }

    #[test]
    fn an_empty_family_name_is_rejected() {
        assert_eq!(FontFamily::new(""), Err(InvalidFontFamily::Empty));
        assert_eq!(FontFamily::new("   "), Err(InvalidFontFamily::Empty));
    }

    #[test]
    fn a_family_name_with_markup_characters_is_rejected() {
        for name in ["Sans\"; #set page(width: 1cm); \"", "Sans#", "Sans$x$"] {
            assert!(
                matches!(
                    FontFamily::new(name),
                    Err(InvalidFontFamily::ForbiddenCharacter { .. })
                ),
                "{name:?} must not be accepted as a family name"
            );
        }
    }

    #[test]
    fn a_family_name_with_a_newline_is_rejected() {
        assert!(matches!(
            FontFamily::new("Sans\nSerif"),
            Err(InvalidFontFamily::ForbiddenCharacter { character: '\n' })
        ));
    }

    #[test]
    fn an_overlong_family_name_is_rejected() {
        let name = "a".repeat(FontFamily::MAX_LENGTH + 1);

        assert_eq!(
            FontFamily::new(&name),
            Err(InvalidFontFamily::TooLong {
                length: FontFamily::MAX_LENGTH + 1
            })
        );
    }

    #[test]
    fn rejection_names_the_offending_character() {
        let error = FontFamily::new("Sans#").unwrap_err();

        assert!(
            error.to_string().contains('#'),
            "message must show the offending character, got: {error}"
        );
    }

    #[test]
    fn typography_reports_what_it_was_built_from() {
        let typography = Typography::new(
            FontFamily::new("DejaVu Sans").unwrap(),
            points(11.0),
            points(15.0),
        );

        assert_eq!(typography.family().as_str(), "DejaVu Sans");
        assert!((typography.size().points() - 11.0).abs() < f64::EPSILON);
        assert!((typography.leading().points() - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn an_ordinary_language_tag_is_accepted() {
        assert_eq!(LanguageTag::new("en").unwrap().as_str(), "en");
        assert_eq!(LanguageTag::new("pt-BR").unwrap().as_str(), "pt-br");
    }

    #[test]
    fn a_language_tag_is_normalised_to_lower_case() {
        // Two spellings of one language must not produce two different
        // outputs, which the determinism requirement would notice.
        assert_eq!(
            LanguageTag::new("EN-GB").unwrap(),
            LanguageTag::new("en-gb").unwrap()
        );
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_from_a_language_tag() {
        assert_eq!(LanguageTag::new("  de  ").unwrap().as_str(), "de");
    }

    #[test]
    fn an_empty_language_tag_is_rejected() {
        assert_eq!(LanguageTag::new("   "), Err(InvalidLanguageTag::Empty));
    }

    #[test]
    fn an_overlong_language_tag_is_rejected() {
        let error = LanguageTag::new(&"a".repeat(LanguageTag::MAX_LENGTH + 1)).unwrap_err();

        assert!(
            error
                .to_string()
                .contains(&LanguageTag::MAX_LENGTH.to_string()),
            "the message must state the bound, got: {error}"
        );
    }

    #[test]
    fn a_language_tag_with_markup_characters_is_rejected() {
        // The tag reaches the output as part of an instruction, so its
        // alphabet is closed for the same reason a family name's is.
        for tag in ["en\"", "en)", "en 1", "en_gb"] {
            assert!(
                LanguageTag::new(tag).is_err(),
                "{tag:?} must not be accepted"
            );
        }
    }

    #[test]
    fn language_tag_rejection_names_the_offending_character() {
        let error = LanguageTag::new("en_gb").unwrap_err();

        assert_eq!(
            error,
            InvalidLanguageTag::ForbiddenCharacter { character: '_' }
        );
        assert!(error.to_string().contains('_'), "got: {error}");
    }

    #[test]
    fn avoidance_defaults_to_weighing_the_avoidance_normally() {
        assert_eq!(Avoidance::default().percent(), 100);
    }

    #[test]
    fn avoidance_accepts_the_whole_permitted_range() {
        for percent in [0, 1, 100, Avoidance::MAX_PERCENT] {
            assert_eq!(Avoidance::from_percent(percent).unwrap().percent(), percent);
        }
    }

    #[test]
    fn an_avoidance_beyond_the_bound_is_rejected() {
        let error = Avoidance::from_percent(Avoidance::MAX_PERCENT + 1).unwrap_err();

        assert_eq!(
            error,
            InvalidAvoidance {
                percent: Avoidance::MAX_PERCENT + 1
            }
        );
        assert!(
            error
                .to_string()
                .contains(&Avoidance::MAX_PERCENT.to_string()),
            "the message must state the bound, got: {error}"
        );
    }

    #[test]
    fn typography_is_unjustified_and_language_less_by_default() {
        let typography = Typography::new(
            FontFamily::new("DejaVu Sans").unwrap(),
            points(11.0),
            points(15.4),
        );

        assert!(!typography.is_justified());
        assert!(
            typography.language().is_none(),
            "a language nobody named would turn on hyphenation nobody asked for"
        );
        assert_eq!(typography.avoidance(), Avoidance::DEFAULT);
    }

    #[test]
    fn typography_remembers_what_a_theme_asked_for() {
        let typography = Typography::new(
            FontFamily::new("DejaVu Sans").unwrap(),
            points(11.0),
            points(15.4),
        )
        .justified(true)
        .with_language(LanguageTag::new("en").unwrap())
        .with_avoidance(Avoidance::from_percent(250).unwrap());

        assert!(typography.is_justified());
        assert_eq!(typography.language().map(LanguageTag::as_str), Some("en"));
        assert_eq!(typography.avoidance().percent(), 250);
    }
}
