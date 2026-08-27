//! Frameworks ring: the adapters that speak to the host.
//!
//! The filesystem and the wall clock are external agencies like any other
//! engine, and this crate is the only one that names them. Keeping the two
//! modules that call `std::fs` alone here is what makes "no filesystem call
//! happens outside the sandbox" a claim about one small crate.
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
pub mod path_resolver;
pub mod source_store;

pub use path_resolver::FilesystemPathResolver;
