## Why

The walking skeleton established a quality gate: formatting, lints, tests, the
architecture guard, a WASM build. That gate checks whether the code is
*correct-looking*. It says nothing about whether the code is *tested*, whether
its dependencies are safe to ship, or whether the project may legally be
distributed at all.

Three gaps are worth naming plainly:

- **The project ships no LICENSE file.** Every crate manifest declares
  `license = "Apache-2.0"`, and `LICENSING.md` explains why that is forced by
  the embedded engine — but the licence text itself is not in the repository.
  A declaration without the text is a claim the repository cannot back up.
- **Nothing measures coverage.** 230 tests is a number, not a guarantee. Which
  branches are exercised is currently unknown, so the suite's real reach is
  unknown too.
- **Nothing checks the dependency tree.** `architecture.toml` governs which
  layer may depend on what, but not whether a dependency has a known
  vulnerability, carries an incompatible licence, or is declared and never used.

The cost of closing these grows with the codebase. Each one is cheapest to
establish now, while there are seven thousand lines to bring into line rather
than seventy thousand.

## What Changes

**Licensing paperwork.** The Apache-2.0 licence text is added as `LICENSE`, and
the licence allow-list becomes machine-enforced rather than prose: a dependency
whose licence is not on the list fails the build. This turns the obligation
`LICENSING.md` describes into something that cannot quietly regress.

**Dependency hygiene.** Three checks join the gate:

- **`cargo-audit`** — known RUSTSEC security advisories against the dependency
  tree,
- **`cargo-deny`** — licence policy, as above, plus duplicate and banned crates,
- **`cargo-machete`** — declared-but-unused dependencies, which the architecture
  guard cannot see because it only checks direction.

**Lints stay mandatory.** `cargo clippy --workspace --all-targets -- -D warnings`
is already a gate job and remains one. This change does not add clippy; it
sharpens it, with thresholds for function length and cognitive complexity, and
with the additional lint groups described under Static analysis below.

**Documentation completeness.** The documentation build must complete with no
warnings, so a broken intra-doc link or an undocumented public item fails the
build rather than accumulating.

**Static analysis.** Code quality is enforced by analyzers that already exist,
configured properly, rather than by anything written here:

- clippy's `nursery`, `cargo` and selected `restriction` groups join `pedantic`,
- rustc's own lint groups — `rust_2018_idioms`, `future_incompatible`,
  `missing_debug_implementations`, `trivial_casts` and the rest,
- `rustfmt.toml`, so formatting is a stated policy rather than whatever the
  defaults happen to be,
- `shellcheck` on the gate script itself, which is currently unlinted shell,
- `taplo` on the TOML files, which now number six,
- `typos` on source and documentation,
- `cargo-hack`, so every feature combination is known to compile,
- `cargo-msrv`, so the declared `rust-version` is verified rather than asserted.

No custom checker is written. The one custom guard this project has — the
architecture guard — stays, because the layer table is particular to this
codebase and no tool can know it.

**A coverage floor.** Line coverage is measured across the whole workspace and
must be at least 90%. Where the current suite falls short, tests are added to
reach the floor; the floor is not lowered to meet the suite.

**Repository hygiene.** `.gitignore` is expanded beyond its single `/target`
line to cover coverage artefacts, editor and OS files, and profiling output.

## Capabilities

### New Capabilities

None. This change alters the build, the tooling and the repository's paperwork.
It does not change what `adocpdf` does when a document is rendered.

### Modified Capabilities

None. Every requirement in `document-rendering`, `theming` and
`project-sandbox` holds exactly as written, before and after. `skip_specs` is
set in `.openspec.yaml` accordingly, rather than inventing a requirement to
satisfy validation.

## Non-goals

- **Not a CI pipeline.** The gate script is the deliverable. Wiring it into a
  hosted CI service is a separate concern with its own decisions about runners,
  caching and secrets.
- **Not a coverage ratchet.** The floor is a fixed 90%, not a
  never-decreasing high-water mark that rises automatically. A ratchet is worth
  considering later, once the floor has held for a while.
- **Not mutation testing.** Coverage measures what ran, not whether the
  assertions would catch a defect. That is a stronger and more expensive check,
  and a prerequisite for it is a suite that runs everything at all.
- **Not a formatting overhaul.** `rustfmt.toml` states the policy the defaults
  already produce; it does not reformat the codebase to a new house style.
- **Not vendoring the advisory database.** The advisory check needs network
  access when it runs. Making it work offline is a CI concern, per the first
  non-goal.
- **No new product behaviour, and no relaxing of existing behaviour to make a
  check pass.** If a quality check and a specified behaviour conflict, the
  specified behaviour wins and the check is adjusted.

## Impact

**Tooling, not layers.** No crate gains a runtime dependency. The layer table is
untouched, and the architecture guard must continue to pass unchanged.

**`xtask` is unchanged.** It keeps the architecture guard and gains nothing.

**New developer tools required.** `cargo-llvm-cov`, `cargo-audit`, `cargo-deny`,
`cargo-machete`, `typos-cli`, `taplo-cli`, `cargo-hack` and `shellcheck` are not
currently installed on the build machine. The gate must
fail with a usable message when a tool is missing rather than silently skipping
the job — a check that is skipped when absent is worse than no check, because it
reports success.

**Existing files change rather than being replaced.** `README.md`,
`LICENSING.md`, `AGENTS.md`, `scripts/ci/gate.sh`, `clippy.toml` and
`.gitignore` all already exist and are edited in place.

**A stricter lint set will surface findings in existing code.** Those are fixed,
not allowed. Reaching 90% coverage may also mean new tests.

**Decisions re-opened.** Two, both within this change and both recorded in
design.md. The custom file-length guard specified by the original D1 was built,
then withdrawn and deleted: clippy's `too_many_lines` already covers the real
concern, and 460 lines of bespoke checker crowded out the analyzers that catch
more for nothing. D5's refusal to adopt `clippy::nursery` is reversed with it —
the objection was toolchain churn, and `rust-toolchain.toml` pins the compiler.

The Apache-2.0 obligation, the layer table and the conventions all stand
untouched; this change enforces them mechanically rather than revisiting them.
