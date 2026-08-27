//! The AsciiDoc adapter: source text in, the document model out.
//!
//! This is the only crate that names `asciidoc-parser`, and
//! `architecture.toml` is what makes that a checked claim rather than a
//! convention. The upstream parser is wrapped here so that swapping it would
//! touch this crate and nothing else, and its errors are mapped to typed
//! domain errors at the boundary, so a foreign error type never travels
//! inward.
//!
//! [`parser`] turns a document into the model; [`inline`] turns the parser's
//! inline substitutions into styled runs, and owns both predicates that refuse
//! source which could forge that structure.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod inline;
pub mod parser;
