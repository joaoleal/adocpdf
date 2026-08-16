//! The fonts the renderer can set text in.
//!
//! Fonts are embedded in the binary rather than read from the host. A renderer
//! that searched the system for fonts would produce different output on
//! different machines, which the determinism requirement forbids; it would also
//! have nothing to find under WASM, where there is no font directory.
//!
//! The trade-off is binary size and a fixed repertoire. Both are acceptable for
//! a walking skeleton, and widening the repertoire later is additive.

use typst::foundations::Bytes;
use typst::text::{Font, FontBook};

/// The body face, compiled into the binary.
///
/// DejaVu Sans, under the Bitstream Vera licence — see
/// `assets/fonts/LICENSE-DejaVu.txt`, which must be distributed with any copy
/// of this software.
const BODY_FONT: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");

/// The family name the embedded body face reports.
///
/// The default theme names this family, so the two must agree: a theme naming a
/// family no embedded font provides would silently fall back.
pub const BODY_FAMILY: &str = "DejaVu Sans";

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
        let data = Bytes::new(BODY_FONT);
        let font = Font::new(data, 0).expect("the embedded body font is a valid font file");

        Self {
            book: FontBook::from_fonts([&font]),
            fonts: vec![font],
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

    #[test]
    fn the_body_font_loads_into_a_font_book() {
        let fonts = EmbeddedFonts::load();

        assert_eq!(fonts.len(), 1);
        assert!(fonts.font(0).is_some());
    }

    #[test]
    fn the_book_knows_the_embedded_family_by_name() {
        let fonts = EmbeddedFonts::load();

        let known: Vec<&str> = fonts.book().families().map(|(name, _)| name).collect();

        assert!(
            known
                .iter()
                .any(|name| name.eq_ignore_ascii_case(BODY_FAMILY)),
            "the book must expose the family the default theme names, got: {known:?}"
        );
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
