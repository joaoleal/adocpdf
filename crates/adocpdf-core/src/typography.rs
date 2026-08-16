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

/// How text is set.
///
/// Leading is the distance between consecutive baselines. It is a length rather
/// than a multiple of the font size so that the value written to the output is
/// the value stated here, with no arithmetic in between to disagree about.
#[derive(Debug, Clone, PartialEq)]
pub struct Typography {
    family: FontFamily,
    size: Length,
    leading: Length,
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
            size,
            leading,
        }
    }

    /// The font family.
    #[must_use]
    pub fn family(&self) -> &FontFamily {
        &self.family
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(value: f64) -> Length {
        Length::from_points(value).unwrap()
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
}
