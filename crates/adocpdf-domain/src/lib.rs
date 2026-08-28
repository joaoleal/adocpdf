//! Entities, value objects, ports and use cases.
//!
//! This layer states the business rules of rendering an AsciiDoc document, and
//! the traits ([ports]) through which it reaches the outside world. It must
//! never name Typst, the AsciiDoc parser, the filesystem, or a delivery
//! mechanism — those are implementation choices that live in the outer rings.
//!
//! [ports]: https://en.wikipedia.org/wiki/Hexagonal_architecture_(software)
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod document_plan;
pub mod error;
pub mod ports;
pub mod render_document;
pub mod sandbox;

pub use document_plan::{LayoutPlan, PlanItem, plan_document};
pub use error::{DomainError, SourceLocation};
pub use ports::{
    Clock, Date, DocumentParser, DocumentRenderer, InvalidDate, ParseOutcome, SkippedConstruct,
    SourceStore, ThemeRepository,
};
pub use render_document::{RenderDocument, RenderReport, RenderRequest};
pub use sandbox::{PathResolver, ProjectRoot, ResolutionError, SandboxedPath};
