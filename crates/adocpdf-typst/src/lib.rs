//! Adapters implementing the domain's ports with the Typst engine.
//!
//! This is the only crate that names Typst: the markup emitter, the embedded
//! font book, the in-memory world the engine reads, and the renderer that
//! typesets a plan and writes PDF bytes. Adapters catch external errors at the
//! boundary and map them to typed domain errors, so a foreign error type never
//! travels inward.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod emitter;
pub mod fonts;
pub mod markup;
pub mod renderer;
pub mod themes;
pub mod world;
