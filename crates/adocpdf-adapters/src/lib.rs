//! Interface adapters.
//!
//! This ring converts between the shapes the outside world speaks and the ones
//! the use case owns. It is the innermost layer that can see both
//! `adocpdf-shared` and `adocpdf-domain` — neither of which may depend on the
//! other — and it names no external technology at all.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod calendar;
pub mod dto;
