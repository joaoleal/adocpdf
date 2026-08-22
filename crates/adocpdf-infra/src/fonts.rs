//! The fonts the renderer can set text in.
//!
//! Fonts are embedded in the binary rather than read from the host. A renderer
//! that searched the system for fonts would produce different output on
//! different machines, which the determinism requirement forbids; it would also
//! have nothing to find under WASM, where there is no font directory.
//!
//! The trade-off is binary size and a fixed repertoire. Both are acceptable,
//! and widening the repertoire later is additive.
//!
//! # Why every variant is embedded rather than synthesised
//!
//! Typst does not fake a missing face. `FontBook::select` resolves a request
//! through `find_best_variant`, which returns the closest face the book
//! actually holds — so asking for bold when only a regular face is embedded
//! yields the regular face, silently, and the text is simply not bold. Every
//! variant the renderer can ask for therefore has to be present.

use typst::foundations::Bytes;
use typst::text::{Font, FontBook};

/// The faces compiled into the binary.
///
/// All are DejaVu 2.37, under the Bitstream Vera licence — see
/// `assets/fonts/LICENSE-DejaVu.txt`, which must be distributed with any copy
/// of this software.
const EMBEDDED_FACES: [&[u8]; 6] = [
    include_bytes!("../assets/fonts/DejaVuSans.ttf"),
    include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf"),
    include_bytes!("../assets/fonts/DejaVuSans-Oblique.ttf"),
    include_bytes!("../assets/fonts/DejaVuSans-BoldOblique.ttf"),
    include_bytes!("../assets/fonts/DejaVuSansMono.ttf"),
    include_bytes!("../assets/fonts/DejaVuSansMono-Bold.ttf"),
];

/// The family name the embedded body faces report.
///
/// The default theme names this family, so the two must agree: a theme naming a
/// family no embedded font provides would silently fall back.
pub const BODY_FAMILY: &str = "DejaVu Sans";

/// The family name the embedded monospace faces report.
///
/// Verbatim content and monospaced inline text are set in this family, so a
/// theme's monospace family must resolve to it for the same reason.
pub const MONOSPACE_FAMILY: &str = "DejaVu Sans Mono";

/// The fonts available to a render, and the index the engine looks them up by.
#[derive(Debug, Clone)]
pub struct EmbeddedFonts {
    book: FontBook,
    fonts: Vec<Font>,
}

impl EmbeddedFonts {
    /// Loads the embedded fonts.
    ///
    /// # Panics
    ///
    /// Panics if the embedded font data cannot be parsed. That would mean the
    /// binary itself is malformed — the bytes are compiled in, so there is no
    /// input that could cause it and no recovery that would make sense.
    #[must_use]
    #[allow(
        clippy::expect_used,
        reason = "the data is compiled into the binary; a failure here means the build is \
                  corrupt, which no caller could handle"
    )]
    pub fn load() -> Self {
        let fonts: Vec<Font> = EMBEDDED_FACES
            .iter()
            .map(|face| {
                Font::new(Bytes::new(*face), 0).expect("an embedded face is a valid font file")
            })
            .collect();

        Self {
            book: FontBook::from_fonts(&fonts),
            fonts,
        }
    }

    /// Metadata about every available font.
    #[must_use]
    pub fn book(&self) -> &FontBook {
        &self.book
    }

    /// The font at `index`, if there is one.
    ///
    /// Returns `None` for an out-of-range index rather than panicking: the
    /// engine may ask with an index from a different font book during
    /// incremental compilation, and that is not an error.
    #[must_use]
    pub fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    /// Whether any embedded face reports this family name.
    ///
    /// Compared case-insensitively, as the engine's own lookup is.
    #[must_use]
    pub fn provides_family(&self, family: &str) -> bool {
        self.book
            .families()
            .any(|(name, _)| name.eq_ignore_ascii_case(family))
    }

    /// How many fonts are embedded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// Whether no fonts are embedded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }
}

impl Default for EmbeddedFonts {
    fn default() -> Self {
        Self::load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use typst::text::{FontStretch, FontStyle, FontVariant, FontWeight};

    /// The variants the renderer can ask for, and the family each belongs to.
    fn requested_variants() -> [(&'static str, FontStyle, FontWeight); 6] {
        [
            (BODY_FAMILY, FontStyle::Normal, FontWeight::REGULAR),
            (BODY_FAMILY, FontStyle::Normal, FontWeight::BOLD),
            (BODY_FAMILY, FontStyle::Oblique, FontWeight::REGULAR),
            (BODY_FAMILY, FontStyle::Oblique, FontWeight::BOLD),
            (MONOSPACE_FAMILY, FontStyle::Normal, FontWeight::REGULAR),
            (MONOSPACE_FAMILY, FontStyle::Normal, FontWeight::BOLD),
        ]
    }

    #[test]
    fn every_face_loads_into_the_font_book() {
        let fonts = EmbeddedFonts::load();

        assert_eq!(fonts.len(), EMBEDDED_FACES.len());
        for index in 0..fonts.len() {
            assert!(fonts.font(index).is_some(), "face {index} must load");
        }
    }

    #[test]
    fn the_book_knows_both_embedded_families_by_name() {
        let fonts = EmbeddedFonts::load();

        let known: Vec<&str> = fonts.book().families().map(|(name, _)| name).collect();

        for family in [BODY_FAMILY, MONOSPACE_FAMILY] {
            assert!(
                known.iter().any(|name| name.eq_ignore_ascii_case(family)),
                "the book must expose {family}, got: {known:?}"
            );
        }
    }

    #[test]
    fn every_requested_variant_resolves_to_a_distinct_face() {
        let fonts = EmbeddedFonts::load();

        let mut resolved = Vec::new();
        for (family, style, weight) in requested_variants() {
            let variant = FontVariant::new(style, weight, FontStretch::NORMAL);
            let index = fonts
                .book()
                .select(&family.to_lowercase(), variant)
                .unwrap_or_else(|| panic!("{family} {style:?} {weight:?} must resolve to a face"));

            assert!(
                !resolved.contains(&index),
                "{family} {style:?} {weight:?} resolved to a face already used by another \
                 variant; Typst does not synthesise, so a shared face means that variant \
                 renders identically to the other"
            );
            resolved.push(index);
        }
    }

    #[test]
    fn the_embedded_family_matches_the_default_theme() {
        assert_eq!(
            BODY_FAMILY,
            adocpdf_core::theme::DEFAULT_FONT_FAMILY,
            "a theme naming a family no embedded font provides would silently fall back"
        );
    }

    #[test]
    fn an_index_past_the_end_yields_no_font() {
        let fonts = EmbeddedFonts::load();

        assert!(
            fonts.font(99).is_none(),
            "the engine may ask with an index from a different book; that is not an error"
        );
    }
}
