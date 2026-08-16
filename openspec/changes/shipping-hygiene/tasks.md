## 1. Precondition

- [x] 1.1 Confirm `bootstrap-repository` has been applied: `.github/workflows/`
      exists and `git log` shows commits. *Verified by:* both are true. If
      either is not, stop — `committed` has nothing to lint and `git-cliff`
      nothing to summarise against an empty history, and CI files would have no
      workflow directory to live in (proposal, Ordering).

## 2. Build profiles, decided and measured

- [x] 2.1 Record a baseline: wall-clock for a clean `cargo build --release` and
      a clean `cargo test --workspace`, and the size of the release binary.
      *Verified by:* three numbers are reported, each from a cold `target/`, so
      the comparisons in 2.2 and 2.3 mean something.
- [x] 2.2 Add `codegen-units = 1` to `[profile.release]`, leaving `lto` at
      `"thin"` (design D2). *Verified by:* the release build succeeds and its
      new time and binary size are reported against the 2.1 baseline. `lto` is
      not touched.
- [x] 2.3 Add `[profile.dev.package."*"] opt-level = 2` and re-measure a clean
      `cargo test --workspace` (design D5). *Verified by:* before-and-after
      numbers are reported. **If the improvement does not materialise, remove
      the setting** rather than keeping it — the task is to find out, not to
      apply it.
- [x] 2.4 Confirm the dev-profile change moves neither the coverage figure nor
      the MSRV job's result. *Verified by:* `scripts/ci/gate.sh` passes and the
      coverage percentage is reported alongside the figure from before the
      change.

## 3. An auditable artefact

- [x] 3.1 Install `cargo-auditable` 0.7.5 and build the release binary with it.
      *Verified by:* `cargo audit bin` against the produced binary lists the
      dependency graph.
- [x] 3.2 Check whether `cargo-auditable` alters the dependency graph, by
      comparing `cargo tree` before and after and re-running the architecture
      guard and `cargo deny check`. *Verified by:* the comparison is reported.
      If a new crate has entered the graph, it is named and checked against
      `deny.toml` and `architecture.toml` rather than passed over.
- [x] 3.3 Add `strip = "debuginfo"` to `[profile.release]` (design D3), then
      rebuild with `cargo-auditable` and confirm the embedded data survived.
      *Verified by:* `cargo audit bin` still reads the manifest from the
      stripped binary. **If it does not, remove `strip`** — D3 states the
      tie-breaker: auditability wins over size.
- [x] 3.4 Record in `Cargo.toml`, beside the profile, why `panic = "abort"` is
      absent (design D4). *Verified by:* the comment states all three reasons,
      so the omission reads as a decision; `taplo fmt --check` still passes.

## 4. SBOM

- [x] 4.1 Install `cargo-cyclonedx` 0.5.9 and generate an SBOM for the
      workspace. *Verified by:* the output is valid CycloneDX and lists the
      dependencies that `cargo tree` shows.
- [x] 4.2 Add SBOM generation to a release CI job, publishing the result as a
      release artefact and **not** committing it (design D8). *Verified by:*
      `actionlint` and `zizmor` pass on the workflow, and `.gitignore` covers
      the generated file so it cannot be committed by accident.

## 5. Commit convention

- [x] 5.1 Add `committed` 1.1.11 with its configuration, and run it against the
      existing history. *Verified by:* the tool runs and its findings are
      reported. If commits created by `bootstrap-repository` fail their own
      convention, say so plainly rather than adjusting the configuration to
      accept them.
- [x] 5.2 Add a CI job running `committed` over a pull request's commit range,
      installed via `cargo install committed --locked` since
      `taiki-e/install-action` does not carry it (design D6). *Verified by:*
      `actionlint` and `zizmor` pass, and the job's range covers the PR's
      commits rather than only `HEAD`.
- [x] 5.3 Document in `AGENTS.md` and `CONTRIBUTING.md` that this is the one
      check not in `scripts/ci/gate.sh`, why — the gate checks the working tree
      and this checks history — and the local command to run before opening a
      pull request. *Verified by:* the gate's job count is unchanged at
      seventeen and the documented reason matches that fact.

## 6. Changelog

- [x] 6.1 Add `cliff.toml` and generate `CHANGELOG.md` from the existing
      history. *Verified by:* the changelog's entries correspond to real
      commits; nothing is invented for a release that has not happened.
- [x] 6.2 Document that the changelog is regenerated when a release is cut, not
      per commit (design D6). *Verified by:* the instruction is written down and
      no CI job regenerates it on push.

## 7. Close out

- [x] 7.1 Run `scripts/ci/gate.sh` and confirm it passes. *Verified by:* the
      gate prints `gate passed`.
- [x] 7.2 Report which settings survived measurement and which were removed,
      naming any that were reverted under 2.3 or 3.3. *Verified by:* the summary
      distinguishes what was measured from what was reasoned about, and states
      the numbers.
