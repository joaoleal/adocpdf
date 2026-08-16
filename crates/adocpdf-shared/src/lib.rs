//! Boundary data transfer objects.
//!
//! These types cross the edge between a delivery mechanism and the application.
//! They carry no business rules, so a delivery layer — the WASM surface in
//! particular — can describe a render request without linking the domain.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod render;

pub use render::{RenderReportDto, RenderRequestDto, SkippedConstructDto};
