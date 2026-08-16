## 1. Admit the dependency, deliberately

- [ ] 1.1 Add `proptest` 1.11.0 to `[workspace.dependencies]` and as a
      dev-dependency of `adocpdf-domain` and `adocpdf-infra`, and name it in the
      `external` list of both crates in `architecture.toml`. *Verified by:*
      `cargo test -p xtask --test architecture` passes — and, before the
      `architecture.toml` edit, fails with a `ForbiddenExternal` violation,
      confirming the guard sees dev-dependencies as the file claims.
- [ ] 1.2 Survey what `proptest` drags in: run `cargo deny check licenses` and
      `cargo audit` against the new tree and report every crate added and its
      licence. *Verified by:* both commands pass unchanged. If a transitive
      crate needs a licence outside `deny.toml`'s allow-list, stop and report it
      — widening the allow-list is a decision for the user, not part of this
      task.

## 2. The injection boundary as an invariant

- [ ] 2.1 Add Tier 1 property tests for `markup::string_literal` (design D2):
      for any input string, the output opens and closes with an unescaped quote,
      carries no unescaped quote or backslash between them, contains no raw
      control character, and decodes back through a test-local inverse decoder
      to exactly the input. *Verified by:* the properties pass over the
      configured case count, and each one fails when the corresponding arm of
      `string_literal` is temporarily broken — a property that cannot fail is
      not evidence.
- [ ] 2.2 Add the Tier 2 property: emit the literal, compile it through the real
      rendering path, and assert the text that comes back equals the input, at a
      low case count. *Verified by:* the property passes, and its runtime is
      measured and reported so the case count is a decision rather than a
      guess.
- [ ] 2.3 Commit the `proptest-regressions/` directory and confirm it is not
      excluded by `.gitignore` (design D4). *Verified by:* `git check-ignore -v`
      reports no match for the regressions path, and a deliberately induced
      failure writes a file there that a later run replays.

## 3. The sandbox rule as an invariant

- [ ] 3.1 Add a property test in `adocpdf-domain` generating path expressions
      from ordinary segments, `.`, `..`, absolute prefixes and empty components,
      driven through a hand-written in-memory fake `PathResolver`, asserting
      that acceptance depends only on where a path resolves — two expressions
      resolving to the same location are accepted or refused alike (design D3).
      *Verified by:* the property passes, and the existing example-based sandbox
      tests still pass unchanged; no existing test is weakened or deleted to
      accommodate it.
- [ ] 3.2 Confirm the symlink cases remain integration tests in `adocpdf-infra`
      against real I/O, and are not folded into the domain property. *Verified
      by:* `crates/adocpdf-infra/tests/sandbox.rs` still exercises real symlinks
      and still passes.

## 4. Fuzzing

- [ ] 4.1 Run `cargo fuzz init`, then immediately run the architecture guard and
      `cargo deny check` to establish whether a root-level `fuzz/` directory is
      visible to either — recorded as unverified in design D6. *Verified by:*
      both tools are run and their actual behaviour is reported. If either does
      see `fuzz/`, stop and report it, because D6's reasoning depends on the
      answer.
- [ ] 4.2 Record in `deny.toml` and `architecture.toml` that `fuzz/` is outside
      each policy, why, that `libfuzzer-sys` is `(MIT OR Apache-2.0) AND NCSA`
      and would fail the allow-list if it were in scope, and the condition that
      should reopen the decision (design D6). *Verified by:* both files carry
      the note, and `taplo fmt --check` still passes.
- [ ] 4.3 Add a fuzz target over the pure parse → plan → emit path taking
      arbitrary bytes as an AsciiDoc source, with no file read, no file written
      and no PDF laid out (design D7). *Verified by:* `cargo +nightly fuzz run`
      executes the target; the target's source contains no filesystem call.
- [ ] 4.4 Run the target for a bounded budget and report what it finds, with
      each finding triaged as this project's defect or upstream
      `asciidoc-parser`'s. *Verified by:* the run completes and its findings are
      reported. Findings are **not** fixed here — per the proposal's non-goals,
      a real defect is its own change.
- [ ] 4.5 Convert every crash the run found into a minimised `#[test]` in the
      ordinary stable suite (design D5). *Verified by:* each new test reproduces
      its crash on the pinned stable toolchain with no nightly involved. A test
      that currently fails is committed as a documented failing test rather than
      deleted or weakened — `AGENTS.md` forbids the alternatives.

## 5. Mutation testing

- [ ] 5.1 Install `cargo-mutants` 27.1.0 and run it against
      `crates/adocpdf-infra/src/markup.rs` and
      `crates/adocpdf-domain/src/sandbox.rs`, reporting surviving mutants and
      the wall-clock time. *Verified by:* the run completes and the survivor
      count for both files is reported plainly, whatever it is.
- [ ] 5.2 For each survivor in those two files, strengthen the property or add
      the missing assertion until none survive (design D8). *Verified by:* a
      re-run reports zero survivors in both files. If a survivor is genuinely
      equivalent and cannot be killed, it goes in `mutants.toml` with a written
      reason — never silently.
- [ ] 5.3 Run `cargo-mutants` across the rest of the workspace once and report
      the result as information. *Verified by:* the report exists and is
      summarised for the user. No threshold is applied outside the two enforced
      files, and no test is added merely to raise the number.

## 6. Wire it up

- [ ] 6.1 Add a scheduled CI workflow running the fuzz targets on nightly with a
      per-target time budget, installing `cargo-fuzz` via `cargo install
      --locked` since `taiki-e/install-action` does not carry it (design D5).
      *Verified by:* `actionlint` and `zizmor` pass on the workflow. Note this
      task depends on `bootstrap-repository` having created `.github/`.
- [ ] 6.2 Add a scheduled CI workflow running `cargo-mutants`, failing on any
      survivor in the two enforced files and publishing the workspace report as
      an artifact (design D8). *Verified by:* `actionlint` and `zizmor` pass;
      the failure condition is scoped to those two files and to nothing else.
- [ ] 6.3 Document in `AGENTS.md` and `README.md` what each of the three
      instruments checks, where it runs, and what it does not prove — in
      particular that fuzzing needs nightly and is not part of the gate.
      *Verified by:* every command quoted runs as written, and no claim is made
      that the gate covers fuzzing or mutation testing.
- [ ] 6.4 Run `scripts/ci/gate.sh` and confirm it still passes, including the
      coverage floor, which stays at 90% (design D9). *Verified by:* the gate
      prints `gate passed`, and the coverage figure is reported before and after
      so the effect of the new tests is visible rather than assumed.
