//! Laying a plan out with the embedded engine and exporting PDF bytes.

use adocpdf_domain::document_plan::LayoutPlan;
use adocpdf_domain::error::DomainError;
use adocpdf_domain::ports::{Date, DocumentRenderer};
use typst::diag::{EcoVec, SourceDiagnostic};
use typst_layout::PagedDocument;

use crate::emitter::emit;
use crate::fonts::EmbeddedFonts;
use crate::world::InMemoryWorld;

/// Renders through an embedded Typst.
///
/// The engine is a library call, not a subprocess: nothing is written to a
/// temporary file, no binary is shelled out to, and the whole render happens in
/// this process. That is what makes it plausible to run under WASM later, and
/// what will make incremental rendering possible at all.
#[derive(Debug, Clone)]
pub struct TypstRenderer {
    fonts: EmbeddedFonts,
}

impl TypstRenderer {
    /// Creates a renderer with the embedded fonts.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fonts: EmbeddedFonts::load(),
        }
    }
}

impl Default for TypstRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentRenderer for TypstRenderer {
    fn render(&self, plan: &LayoutPlan, origin: &str, today: Date) -> Result<Vec<u8>, DomainError> {
        let world = InMemoryWorld::new(emit(plan), self.fonts.clone(), today);

        // Warnings are discarded rather than reported. Every one would be about
        // markup this crate generated, not about anything the author wrote, so
        // surfacing them would blame the author for our output.
        let compiled = typst::compile::<PagedDocument>(&world);

        let document = compiled
            .output
            .map_err(|diagnostics| DomainError::LayoutFailed {
                path: origin.to_owned(),
                reason: describe(&diagnostics),
            })?;

        typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|diagnostics| {
            DomainError::LayoutFailed {
                path: origin.to_owned(),
                reason: describe(&diagnostics),
            }
        })
    }
}

/// Reduces engine diagnostics to one message.
///
/// Only the messages are kept: the spans point into markup this crate
/// generated, so reporting them would give the author positions in a file they
/// never wrote.
fn describe(diagnostics: &EcoVec<SourceDiagnostic>) -> String {
    if diagnostics.is_empty() {
        return "the layout engine failed without saying why".to_owned();
    }

    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use adocpdf_core::document::{Block, Document, InlineText, Paragraph};
    use adocpdf_core::theme::ThemeSet;
    use adocpdf_domain::document_plan::plan_document;

    use super::*;

    fn a_date() -> Date {
        Date::new(2026, 8, 16).unwrap()
    }

    fn plan_of(document: &Document) -> LayoutPlan {
        plan_document(document, &ThemeSet::default()).unwrap()
    }

    #[test]
    fn a_document_renders_to_pdf_bytes() {
        let document = Document::new()
            .with_title(InlineText::new("Report"))
            .with_block(Block::Paragraph(Paragraph::new(InlineText::new(
                "The quick brown fox jumps over the lazy dog.",
            ))));

        let bytes = TypstRenderer::new()
            .render(&plan_of(&document), "book.adoc", a_date())
            .expect("a plain document lays out");

        assert!(
            bytes.starts_with(b"%PDF-"),
            "the output must be a PDF, got: {:?}",
            &bytes[..bytes.len().min(16)]
        );
        assert!(bytes.len() > 500, "a real PDF is not a handful of bytes");
    }

    #[test]
    fn the_same_plan_renders_to_the_same_bytes() {
        let document = Document::new()
            .with_title(InlineText::new("Report"))
            .with_block(Block::Paragraph(Paragraph::new(InlineText::new("Body."))));
        let renderer = TypstRenderer::new();

        let first = renderer
            .render(&plan_of(&document), "book.adoc", a_date())
            .unwrap();
        let second = renderer
            .render(&plan_of(&document), "book.adoc", a_date())
            .unwrap();

        assert_eq!(
            first, second,
            "identical input must produce byte-identical output"
        );
    }

    #[test]
    fn an_empty_document_still_renders() {
        let bytes = TypstRenderer::new()
            .render(&plan_of(&Document::new()), "empty.adoc", a_date())
            .expect("an empty document is not an error");

        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn markup_characters_in_content_do_not_break_the_render() {
        let document = Document::new().with_block(Block::Paragraph(Paragraph::new(
            InlineText::new(r#"#set page(width: 1cm) *bold* _under_ $x^2$ ") #("#),
        )));

        let bytes = TypstRenderer::new()
            .render(&plan_of(&document), "hostile.adoc", a_date())
            .expect("content that looks like markup must still render");

        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn a_failure_names_the_document_it_was_laying_out() {
        let error = DomainError::LayoutFailed {
            path: "book.adoc".to_owned(),
            reason: describe(&EcoVec::new()),
        };

        assert!(error.to_string().contains("book.adoc"));
    }

    #[test]
    fn diagnostics_are_reduced_to_their_messages() {
        assert_eq!(
            describe(&EcoVec::new()),
            "the layout engine failed without saying why"
        );
    }
}
