//! WASM/WASI delivery surface.
//!
//! Deliberately empty in this change. It exists so the architecture guard
//! constrains this crate from the first commit, rather than having the rules
//! applied retroactively once a binding surface already exists. See
//! `openspec/changes/render-first-pdf/design.md` — the WASM build is a
//! non-goal here.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]
