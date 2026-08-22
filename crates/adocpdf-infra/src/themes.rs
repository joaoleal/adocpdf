//! The themes a document can name.
//!
//! These are built in, not loaded from a file. A theme *file format* is an
//! explicit non-goal of this change — designing one is its own piece of work,
//! and doing it badly now would be harder to undo than to postpone. But a
//! document still has to be able to name a second theme for per-section theming
//! to mean anything, so a small fixed set ships in the binary.
//!
//! Adding a file-backed repository later is additive: it implements the same
//! port, and nothing above this layer changes.

use adocpdf_core::geometry::{Margins, PageGeometry};
use adocpdf_core::length::Length;
use adocpdf_core::theme::{Theme, ThemeId, ThemeSet, built_in_default_theme};
use adocpdf_core::typography::{FontFamily, Typography};
use adocpdf_domain::error::DomainError;
use adocpdf_domain::ports::ThemeRepository;

use crate::fonts::EmbeddedFonts;

/// A landscape page. Changes page geometry, so it starts a new page.
pub const WIDE: &str = "wide";

/// Larger type on the same page. Changes only typography, so it does not.
pub const LARGE_PRINT: &str = "large-print";

/// Supplies the themes compiled into the binary.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuiltInThemes;

impl BuiltInThemes {
    /// Creates the repository.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl ThemeRepository for BuiltInThemes {
    fn load(&self) -> Result<ThemeSet, DomainError> {
        let default = built_in_default_theme();
        let wide = wide_theme(&default)?;
        let large_print = large_print_theme(&default)?;

        let fonts = EmbeddedFonts::load();
        for (name, theme) in [
            ("the default theme", &default),
            (WIDE, &wide),
            (LARGE_PRINT, &large_print),
        ] {
            ensure_faces_available(name, theme, &fonts)?;
        }

        Ok(ThemeSet::new(default)
            .with(theme_id(WIDE)?, wide)
            .with(theme_id(LARGE_PRINT)?, large_print))
    }
}

/// Refuses a theme naming a family no embedded face provides.
///
/// Typst resolves a font request to the closest face its book holds, so a
/// theme naming a family that is not there does not fail — it silently renders
/// in whatever else is available. Silence is the problem: output would then
/// depend on which faces happen to be embedded, which is the determinism
/// requirement's whole concern.
///
/// # Errors
///
/// Returns [`DomainError::InvalidTheme`] naming the family that could not be
/// provided.
fn ensure_faces_available(
    name: &str,
    theme: &Theme,
    fonts: &EmbeddedFonts,
) -> Result<(), DomainError> {
    let typography = theme.typography();

    for family in [typography.family(), typography.monospace_family()] {
        if !fonts.provides_family(family.as_str()) {
            return Err(DomainError::InvalidTheme {
                id: name.to_owned(),
                reason: format!(
                    "names the font family {:?}, which no embedded face provides; a theme \
                     naming an unavailable family would render in whichever face happened \
                     to be present",
                    family.as_str()
                ),
            });
        }
    }

    Ok(())
}

/// Builds a theme identifier, reporting a bad one as an invalid theme.
fn theme_id(name: &str) -> Result<ThemeId, DomainError> {
    ThemeId::new(name).map_err(|error| DomainError::InvalidTheme {
        id: name.to_owned(),
        reason: error.to_string(),
    })
}

/// A landscape variant of the default: A4 turned on its side.
fn wide_theme(default: &Theme) -> Result<Theme, DomainError> {
    let geometry = PageGeometry::new(
        millimeters(WIDE, 297.0)?,
        millimeters(WIDE, 210.0)?,
        Margins::uniform(millimeters(WIDE, 20.0)?),
    )
    .map_err(|error| DomainError::InvalidTheme {
        id: WIDE.to_owned(),
        reason: error.to_string(),
    })?;

    Ok(Theme::new(geometry, default.typography().clone()))
}

/// The default page, set larger. Geometry is untouched on purpose: this is the
/// theme that proves a typography change does *not* break the page.
fn large_print_theme(default: &Theme) -> Result<Theme, DomainError> {
    let family = FontFamily::new(default.typography().family().as_str()).map_err(|error| {
        DomainError::InvalidTheme {
            id: LARGE_PRINT.to_owned(),
            reason: error.to_string(),
        }
    })?;

    Ok(Theme::new(
        *default.geometry(),
        Typography::new(
            family,
            points(LARGE_PRINT, 16.0)?,
            points(LARGE_PRINT, 22.0)?,
        ),
    ))
}

fn millimeters(theme: &str, value: f64) -> Result<Length, DomainError> {
    Length::from_millimeters(value).map_err(|error| DomainError::InvalidTheme {
        id: theme.to_owned(),
        reason: error.to_string(),
    })
}

fn points(theme: &str, value: f64) -> Result<Length, DomainError> {
    Length::from_points(value).map_err(|error| DomainError::InvalidTheme {
        id: theme.to_owned(),
        reason: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use adocpdf_core::theme::ThemeTransition;

    use super::*;

    fn themes() -> ThemeSet {
        BuiltInThemes::new()
            .load()
            .expect("built-in themes are valid")
    }

    #[test]
    fn the_default_theme_is_available_without_naming_it() {
        assert_eq!(themes().default_theme(), &built_in_default_theme());
    }

    #[test]
    fn both_named_themes_can_be_looked_up() {
        let set = themes();

        assert!(set.get(&ThemeId::new(WIDE).unwrap()).is_some());
        assert!(set.get(&ThemeId::new(LARGE_PRINT).unwrap()).is_some());
    }

    #[test]
    fn the_wide_theme_changes_page_geometry() {
        let set = themes();

        let transition = ThemeTransition::classify(
            set.default_theme(),
            set.get(&ThemeId::new(WIDE).unwrap()).unwrap(),
        );

        assert_eq!(transition, ThemeTransition::PageGeometry);
        assert!(
            transition.forces_page_break(),
            "the wide theme exists to exercise the page-breaking path"
        );
    }

    #[test]
    fn the_large_print_theme_changes_only_typography() {
        let set = themes();

        let transition = ThemeTransition::classify(
            set.default_theme(),
            set.get(&ThemeId::new(LARGE_PRINT).unwrap()).unwrap(),
        );

        assert_eq!(transition, ThemeTransition::TypographyOnly);
        assert!(
            !transition.forces_page_break(),
            "the large-print theme exists to prove restyling does not break the page"
        );
    }

    #[test]
    fn the_wide_theme_is_landscape() {
        let set = themes();
        let wide = set.get(&ThemeId::new(WIDE).unwrap()).unwrap();

        assert!(
            wide.geometry().width() > wide.geometry().height(),
            "wide must be wider than it is tall"
        );
    }

    #[test]
    fn every_named_theme_uses_an_embedded_font() {
        let set = themes();

        for id in set.ids() {
            let theme = set.get(id).unwrap();
            assert_eq!(
                theme.typography().family().as_str(),
                crate::fonts::BODY_FAMILY,
                "theme {id} names a family the renderer cannot embed"
            );
        }
    }

    #[test]
    fn loading_twice_produces_the_same_themes() {
        assert_eq!(themes(), themes(), "themes must not vary between loads");
    }

    #[test]
    fn every_built_in_theme_names_families_the_binary_provides() {
        BuiltInThemes::new()
            .load()
            .expect("the built-in themes must name embedded families");
    }

    #[test]
    fn a_theme_naming_an_unavailable_family_is_rejected() {
        let theme = Theme::new(
            *built_in_default_theme().geometry(),
            Typography::new(
                FontFamily::new("Nonexistent Face").unwrap(),
                Length::from_points(11.0).unwrap(),
                Length::from_points(15.4).unwrap(),
            ),
        );

        let error = ensure_faces_available("test theme", &theme, &EmbeddedFonts::load())
            .expect_err("a family with no face must be refused");

        assert!(
            error.to_string().contains("Nonexistent Face"),
            "the error must name the family, got: {error}"
        );
    }

    #[test]
    fn a_theme_naming_an_unavailable_monospace_family_is_rejected() {
        let theme = Theme::new(
            *built_in_default_theme().geometry(),
            Typography::new(
                FontFamily::new("DejaVu Sans").unwrap(),
                Length::from_points(11.0).unwrap(),
                Length::from_points(15.4).unwrap(),
            )
            .with_monospace_family(FontFamily::new("Nonexistent Mono").unwrap()),
        );

        let error = ensure_faces_available("test theme", &theme, &EmbeddedFonts::load())
            .expect_err("a monospace family with no face must be refused too");

        assert!(error.to_string().contains("Nonexistent Mono"));
    }

    #[test]
    fn the_default_monospace_family_matches_an_embedded_family() {
        assert_eq!(
            adocpdf_core::typography::DEFAULT_MONOSPACE_FAMILY,
            crate::fonts::MONOSPACE_FAMILY,
            "a theme naming a family no embedded font provides would silently fall back"
        );
    }
}
