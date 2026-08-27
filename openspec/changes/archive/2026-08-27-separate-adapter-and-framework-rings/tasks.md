## 1. Close the outbound boundary

- [x] 1.1 Add a characterisation test for the report the binary prints today,
      asserting the exact lines for a render with nothing skipped and for one
      with two skipped constructs — including the `line N, column N` spelling
      and the trailing count on standard error. Write it against the current
      `report_success`, before anything moves. *Verified by:* it passes on the
      unmodified tree, and by deliberately reordering one word in
      `report_success` and seeing it fail — the existing end-to-end tests assert
      only that the output *contains* `out.pdf`, `bytes` and `skipped`, so they
      would not.
- [x] 1.2 Add `crates/adocpdf-cli/src/presenter.rs` formatting
      `RenderReportDto`, make `main` convert through
      `adocpdf_infra::dto::from_domain_report`, and delete `report_success`
      (design D1). *Verified by:* the characterisation test from 1.1 passes
      unchanged; `grep -rn "RenderReport\b" crates/adocpdf-cli/src` finds
      nothing; `from_domain_report` now has a production caller.

## 2. The interface-adapter ring

- [x] 2.1 Create `crates/adocpdf-adapters` and `git mv` `dto.rs` into it, with
      its unit tests. Add its `architecture.toml` entry — workspace
      `adocpdf-domain`, `adocpdf-shared`; external empty — add
      `adocpdf-adapters` to `adocpdf-infra`'s and `adocpdf-cli`'s workspace
      lists, and update the imports in `main.rs`. *Verified by:* `cargo test
      --workspace` passes, and `cargo test -p xtask` passes, which is the guard
      confirming the new crate is governed rather than ungoverned.
- [x] 2.2 `git mv` `unix_timestamp` and `date_from_unix_days` out of `clock.rs`
      into `crates/adocpdf-adapters/src/calendar.rs`, with the two unit tests
      that assert the epoch and the midnight property, and point `parser.rs` and
      `clock.rs` at the new module (design D4). *Verified by:* the moved tests
      pass in their new crate; `crates/adocpdf-infra/src/clock.rs` contains no
      calendar arithmetic; the rendered date in the end-to-end suite is
      unchanged.

## 3. The host ring

- [x] 3.1 Create `crates/adocpdf-host`, `git mv` `clock.rs`, `source_store.rs`
      and `path_resolver.rs` into it along with `tests/sandbox.rs`,
      `tests/source_store.rs` and `tests/support/`, and add its
      `architecture.toml` entry — workspace `adocpdf-domain`,
      `adocpdf-adapters`; external empty. Update `main.rs` and
      `adocpdf-infra/src/lib.rs`. *Verified by:* `cargo test --workspace` and
      `cargo test -p xtask` pass; nothing left in `adocpdf-infra` names
      `std::fs`; the sandbox tests run from their new crate with the same
      assertions.

## 4. The tests that span two technologies

- [x] 4.1 Move `tests/layout/`, its eight includers (`blocks.rs`, `headings.rs`,
      `inline_roles.rs`, `layout_helper.rs`, `list_presentation.rs`, `lists.rs`,
      `paragraph_fill.rs`, `paragraph_presentation.rs`), `tests/hang_regressions.rs`
      and `tests/support_inventory.rs` to `crates/adocpdf-cli/tests/`, changing
      nothing but the crate names in their `use` lines (design D5). Do this
      while `adocpdf-infra` is still one crate, so the move is separable from
      the split. *Verified by:* `cargo test --workspace` passes with the same
      test count; `git diff` over the moved files shows no changed assertion;
      `support_inventory.rs` still resolves `docs/asciidoc-support.md`, which it
      reaches from `CARGO_MANIFEST_DIR` at the same depth.

## 5. The AsciiDoc crate

- [x] 5.1 Create `crates/adocpdf-asciidoc`, `git mv` `parser.rs` and `inline.rs`
      into it with `tests/parser.rs`, `tests/parser_refusal.rs`,
      `tests/known_crashes.rs` and `tests/termination_sweeps.rs`, and add its
      `architecture.toml` entry — workspace `adocpdf-core`, `adocpdf-domain`,
      `adocpdf-adapters`; external `asciidoc-parser` and `proptest`, the latter
      commented as test-only. Remove `asciidoc-parser` from `adocpdf-infra`'s
      manifest and allow-list. *Verified by:* `cargo test --workspace` and
      `cargo test -p xtask` pass; `grep -rn "asciidoc_parser"
      crates/adocpdf-infra` finds nothing; both structure-forgery predicates are
      still defined once, in `inline.rs`, and imported by `parser.rs`.
- [x] 5.2 Point `.github/workflows/mutants.yml` at
      `crates/adocpdf-asciidoc/src/inline.rs`. *Verified by:* the path exists —
      check it with `test -f`, because a stale `--file` argument makes
      `cargo mutants` enforce nothing without failing.
- [x] 5.3 Point `fuzz/Cargo.toml` and `fuzz/fuzz_targets/parse_plan_emit.rs` at
      `adocpdf-asciidoc` for the parser, keeping the emitter import on
      `adocpdf-infra` until task 6.1. *Verified by:* `cargo +nightly fuzz build
      parse_plan_emit` succeeds. The gate never builds `fuzz/`, so this is the
      only thing that checks it.

## 6. What remains becomes the Typst crate

- [x] 6.1 `git mv crates/adocpdf-infra crates/adocpdf-typst`, rename the
      package, and rewrite the `architecture.toml` entry — workspace
      `adocpdf-core`, `adocpdf-domain`; external `typst`, `typst-layout`,
      `typst-pdf`, plus `proptest` and `pdf-extract` commented as test-only.
      Delete the `adocpdf-infra` entry, update `adocpdf-cli` and `adocpdf-wasm`,
      and update `fuzz/` for the emitter (design D8). Drop `adocpdf-adapters`
      and `adocpdf-shared` from both the manifest and the allow-list: both are
      now unused, `parser.rs` and `dto.rs` having been their only consumers.
      *Verified by:* `cargo test
      --workspace`, `cargo test -p xtask` and `cargo +nightly fuzz build` pass;
      `grep -rn "adocpdf.infra" --include='*.rs' --include='*.toml'` finds
      nothing outside `openspec/changes/archive/`; `git log --follow` still
      reaches the history of `markup.rs`.
- [x] 6.2 Update the two configuration files that name a path inside the old
      crate and fail *open* when it is wrong:
      `.github/workflows/mutants.yml`'s `markup.rs` entry, and `_typos.toml`'s
      `crates/adocpdf-infra/assets/` exclusion. *Verified by:* both paths exist;
      the spelling job passes without reading a font binary, and the mutants
      workflow's `--file` arguments both resolve.
- [x] 6.3 Update every remaining prose or data reference to the old crate:
      `LICENSING.md`, `README.md`, `SECURITY.md`, `.cargo/audit.toml`,
      `docs/asciidoc-support.md`, `fuzz/known-crashes.toml` and
      `scripts/ci/known-crashes.sh`. Not every one of these becomes
      `adocpdf-typst`: `scripts/ci/known-crashes.sh:26` points at
      `crates/adocpdf-infra/tests/known_crashes.rs`, and that test moved to
      `adocpdf-asciidoc` in task 5.1 — a blind rename would leave it naming a
      path that does not exist. Check each reference against where the file
      actually went. *Verified by:* `grep -rn "adocpdf-infra"`
      returns only `openspec/changes/archive/`, and the font licence path
      `LICENSING.md` and `README.md` quote resolves to a real file — an
      unresolvable licence path is a licensing defect, not a typo.

## 7. Name the rings in the manifests

- [x] 7.1 Give all nine crate manifests a `description` stating the ring, in
      the shape `adocpdf-core` already uses (design D9). *Verified by:*
      `cargo metadata --format-version 1 --no-deps` reports a ring for every
      crate, and each names the ring `architecture.toml` places it in.

## 8. The layer table, everywhere it is written down

- [x] 8.1 Update the dependency table and the per-crate prose in `AGENTS.md`,
      `CONTRIBUTING.md` and `openspec/config.yaml` to the nine crates, including
      `AGENTS.md`'s claim that `adocpdf-infra` is "the only layer naming an
      external technology" — which becomes a statement about three crates, one
      technology each. *Verified by:* reading each document against
      `architecture.toml`; no sentence may describe a crate that no longer
      exists, and the three tables must agree row for row.
- [x] 8.2 Update `README.md`'s "How it fits together" diagram, which labels four
      steps `(infra)`. *Verified by:* each label names the crate that now holds
      that step, and the spelling job passes.
- [x] 8.3 Update the LikeC4 model: `docs/architecture/model.c4` (the `infra`
      layer, its components, its edges and the two "Confined to adocpdf-infra"
      descriptions), `views.c4` (the `infra` view and the `rings` view's note
      that one crate carries two ring tags), and `docs/architecture/README.md`
      (the file table and the view list). *Verified by:* `make -C
      docs/architecture architecture` renders without error, and the `rings`
      view shows each crate in exactly one ring.
- [x] 8.4 Remove proposals 1, 2 and 3 from `docs/architecture/proposals.md` and
      delete `docs/architecture/proposed.c4`, since all three are now the
      repository rather than a proposal, and fold the two rejected options —
      DTOs in the domain, and an output port — into the record they belong to
      now, this change's `design.md`. *Verified by:* `proposals.md` describes
      only what is still proposed, or is deleted with its README row if nothing
      is; no view references a deleted file.

## 9. The gate

- [x] 9.1 Add `adocpdf-adapters` to `WASM_CLEAN_CRATES` in
      `scripts/ci/gate.sh`. It depends only on `adocpdf-domain` and
      `adocpdf-shared`, so it should build for the browser target — which is the
      claim "the adapter ring names no engine" made checkable. *Verified by:*
      the gate's wasm build job passing with five crates. If it fails, the entry
      comes out and the reason is recorded in `design.md` as a finding, not
      dropped.
- [x] 9.2 Confirm every new manifest declares only what its crate uses.
      *Verified by:* the gate's unused-dependencies job (`cargo-machete`)
      passing — after a split, a leftover dependency is the evidence that a
      module did not really move.
- [x] 9.3 Run `scripts/ci/gate.sh` and confirm all eighteen jobs pass, holding
      the 90% coverage floor without lowering it. *Verified by:* the gate
      prints `gate passed`, and the coverage figure is within noise of the one
      before this change — moving a test between crates cannot change a
      workspace-wide number, so a drop means something was lost in a move.
- [x] 9.4 Confirm the rendered output is unchanged, byte for byte. *Verified
      by:* rendering each fixture in `crates/adocpdf-cli/tests/fixtures/` with
      `--date 2026-08-16` before and after the change and comparing the PDFs
      with `cmp`. No task in this change is allowed to alter a single byte, and
      this is the only check that says so directly.
