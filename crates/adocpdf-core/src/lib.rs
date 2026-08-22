//! The innermost ring of `adocpdf`: the document and theme model.
//!
//! This crate has no dependencies, not even on the rest of the workspace. Every
//! other layer may depend inward on it; it depends on nothing. Anything that
//! needs to name a file, a clock, a parser or a layout engine belongs further
//! out.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod document;
pub mod geometry;
pub mod length;
pub mod theme;
pub mod typography;

pub use document::{
    Admonition, AdmonitionKind, Block, BreakKind, Container, ContainerKind, Document, HeadingLevel,
    InlineNode, InlineStyle, InlineText, InvalidHeadingLevel, List, ListItem, ListKind, Paragraph,
    Quotation, QuotationKind, Section, Verbatim, VerbatimKind,
};
pub use geometry::{InvalidPageGeometry, Margins, PageGeometry};
pub use length::{InvalidLength, Length};
pub use theme::{
    DEFAULT_FONT_FAMILY, InvalidThemeId, Theme, ThemeId, ThemeSet, ThemeTransition,
    built_in_default_theme,
};
pub use typography::{
    Avoidance, DEFAULT_MONOSPACE_FAMILY, FontFamily, InvalidAvoidance, InvalidFontFamily,
    InvalidLanguageTag, LanguageTag, Typography,
};
