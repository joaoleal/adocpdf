//! Adapters implementing the domain's ports.
//!
//! This is the only layer that names an external technology: the AsciiDoc
//! parser, the Typst engine, the filesystem, the host clock. Adapters catch
//! external errors at the boundary and map them to typed domain errors, so a
//! foreign error type never travels inward.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod clock;
pub mod dto;
pub mod emitter;
pub mod fonts;
pub mod inline;
pub mod markup;
pub mod parser;
pub mod path_resolver;
pub mod renderer;
pub mod source_store;
pub mod themes;
pub mod world;

pub use path_resolver::FilesystemPathResolver;
