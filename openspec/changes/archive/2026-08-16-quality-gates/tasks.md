## 1. Paperwork

- [x] 1.1 Add the full Apache-2.0 licence text as `LICENSE` at the repository
      root, and cross-reference it from `LICENSING.md`. *Verified by:* the file
      exists, contains the standard Apache-2.0 text including the appendix, and
      the copyright line names the project.
- [x] 1.2 Expand `.gitignore` beyond `/target` to cover coverage artefacts
      (`*.profraw`, `*.profdata`, `lcov.info`, `target/llvm-cov*`), editor and
      OS files, and profiling output. *Verified by:* `git status --porcelain`
      is empty after a full gate run, which produces coverage artefacts.

## 2. Measure before enforcing

- [x] 2.1 Install `cargo-llvm-cov`, `cargo-audit`, `cargo-deny` and
      `cargo-machete`, and record the exact versions installed in `design.md`
      if they differ from the table there. *Verified by:* each tool reports its
      version.
- [x] 2.2 Measure current line coverage across the workspace and report the
      figure, per crate and overall, without changing any threshold yet.
      *Verified by:* a coverage run completes and the numbers are reported to
      the user. If the overall figure is below 90%, say so plainly rather than
      proceeding as if it were not.
- [x] 2.3 Run `cargo audit`, `cargo deny check licenses` and `cargo machete`
      once against the current tree and report what each finds, before any of
      them becomes a gate job. *Verified by:* all three commands run to
      completion and their findings are reported. An advisory against a
      transitive dependency is reported, not silenced.

## 3. Dependency and documentation checks

- [x] 3.1 Add `deny.toml` with the explicit licence allow-list from design D3,
      plus bans and duplicate policy, with comments explaining why each unusual
      licence is permitted. Per design D2a it carries no `[advisories]` policy —
      advisories are `cargo-audit`'s job. *Verified by:* `cargo deny check
      licenses` passes against the current tree, and removing an allow-listed
      licence makes it fail.
- [x] 3.2 Add the `cargo-audit` security-advisory job to `scripts/ci/gate.sh`,
      failing with the install command when the tool is absent. *Verified by:*
      the job runs against `Cargo.lock` and reports a usable message when
      `cargo-audit` is removed from `PATH`.
- [x] 3.3 Add the `cargo-deny` licence-policy job to the gate, using the same
      tool-presence handling. *Verified by:* the job passes, and fails when an
      allow-listed licence is removed from `deny.toml`.
- [x] 3.4 Add the unused-dependency job using `cargo-machete`, resolving
      anything it reports rather than suppressing it. *Verified by:* the job
      passes with no suppressions, or every suppression carries a comment
      explaining why the dependency is genuinely needed.
- [x] 3.5 Add the documentation job running `cargo doc --workspace --no-deps`
      with warnings denied. *Verified by:* the job passes, and fails when a
      deliberately broken intra-doc link is introduced.

## 4. Static analysis

- [x] 4.1 Remove the custom file-length guard: delete `code-quality.toml`,
      `xtask/src/code_quality.rs`, `xtask/tests/code_quality.rs`, its module
      declaration, and its gate job. *Verified by:* the files are gone,
      `cargo test -p xtask` still passes with the architecture guard intact,
      and the gate no longer names a code-quality job.
- [x] 4.2 Expand `[workspace.lints.rust]` with the rustc lint groups —
      `rust_2018_idioms`, `future_incompatible`, `missing_debug_implementations`,
      `trivial_casts`, `trivial_numeric_casts`, `unused_qualifications`,
      `unused_lifetimes`, `let_underscore` — and fix what they surface rather
      than allowing them. *Verified by:* `cargo clippy --workspace
      --all-targets -- -D warnings` passes with no new `#[allow]` that lacks a
      stated reason.
- [x] 4.3 Add `clippy::nursery`, `clippy::cargo` and the selected `restriction`
      lints (`todo`, `unimplemented`, `dbg_macro`, `indexing_slicing`) to
      `[workspace.lints.clippy]`, and fix what they surface. A lint that is
      genuinely wrong for this codebase is disabled once in `[workspace.lints]`
      with a comment, never scattered as attributes. *Verified by:* clippy
      passes, and every workspace-level exception carries a reason.
- [x] 4.4 Add `rustfmt.toml` stating the formatting policy explicitly.
      *Verified by:* `cargo fmt --all --check` passes without reformatting the
      codebase, so the policy describes what is already there.
- [x] 4.5 Add the `shellcheck` job to the gate for `scripts/ci/gate.sh`, and
      fix what it reports. *Verified by:* the job passes, and reports the
      install command when `shellcheck` is absent.
- [x] 4.6 Add `taplo.toml` and the TOML lint job covering every `.toml` in the
      repository. *Verified by:* the job passes and rejects a deliberately
      malformed TOML file.
- [x] 4.7 Add `_typos.toml` and the spellcheck job over source and
      documentation, correcting real misspellings and allow-listing genuine
      domain terms. *Verified by:* the job passes and flags a deliberately
      introduced misspelling.
- [x] 4.8 Add the `cargo-hack` job checking every feature combination compiles.
      *Verified by:* the job passes across the workspace.
- [x] 4.9 Verify the declared `rust-version = "1.92"` is real with
      `cargo-msrv`, correcting the manifest if the true minimum differs.
      *Verified by:* the reported minimum matches what the manifests declare.

## 5. The coverage floor

- [x] 5.1 Add the coverage job to the gate running
      `cargo llvm-cov --workspace --fail-under-lines 90`, failing with the
      install command when the tool is absent. *Verified by:* the job runs and
      reports a percentage.
- [x] 5.2 Mark genuinely uncoverable code with a coverage-exclusion attribute
      at the point it occurs, each with a comment saying why it cannot be
      tested, per design D4. *Verified by:* every exclusion carries a reason,
      and no exclusion covers code that could be tested with a fake.
- [x] 5.3 Raise coverage to at least 90% by adding tests for the paths the
      measurement in 2.2 showed uncovered, prioritising domain and core over
      delivery. *Verified by:* the coverage job passes at the 90% threshold
      without the threshold having been lowered.

## 6. Documentation

- [x] 6.1 Update `README.md` with the tools a contributor must install, what
      each gate job checks, and the coverage floor. *Verified by:* a reader
      following it from a clean checkout can run the full gate.
- [x] 6.2 Update `AGENTS.md` with the new hard constraints — the coverage
      floor, the licence allow-list, and the expanded lint set — so an agent
      does not discover them by failing the gate. *Verified by:* each new
      check is named, with the file that configures it.
- [x] 6.3 Update `LICENSING.md` to reference the new `LICENSE` file and the
      machine-enforced allow-list. *Verified by:* the licence obligations and
      the mechanism enforcing them are described in one place.

## 7. Final verification

- [x] 7.1 Run the complete gate from a clean target directory and confirm every
      job passes. *Verified by:* `scripts/ci/gate.sh` exits zero, and its
      output names every job that ran — including clippy and the `cargo-audit`
      advisory check, which are required.
