## Why

There is no repository. `git log` reports *"your current branch 'main' does not
have any commits yet"* and `git ls-files` returns zero: the entire project —
7,296 lines of Rust across six crates, a fifteen-job quality gate, the licence
paperwork, and every OpenSpec artifact — exists only as an uncommitted working
tree. A single `git clean` would destroy all of it, and the Conventional Commits
rule in `AGENTS.md` governs a history that has never had a commit to govern.

There is also no CI. `scripts/ci/gate.sh` encodes fifteen jobs and the principle
that a job whose tool is missing *fails* rather than skips, but nothing runs it
except a human who remembers to. Every guarantee the project makes about itself
— the 90% coverage floor, the architecture guard, the licence allow-list, the
MSRV claim — is currently enforced by discipline alone. The `quality-gates`
change built the gate; this change makes it run.

Both gaps have to close together. Committing without CI leaves the gate
optional; adding CI without commits gives the workflow nothing to check out.

## What Changes

- **Version control.** The working tree is committed, in coherent Conventional
  Commits rather than one opaque `chore: initial commit`. `Cargo.lock` is
  tracked: this project ships a binary, and the lockfile is the input that makes
  the determinism requirement and `cargo audit` mean anything.
- **Continuous integration.** A GitHub Actions workflow runs the existing
  `scripts/ci/gate.sh` on push and on pull request. It does not reimplement the
  gate's jobs in YAML — one definition, run in both places, so the local command
  and the merge check cannot drift apart. Gate tools are installed as prebuilt
  binaries rather than compiled from source.
- **The workflow files are themselves linted.** `actionlint` and `zizmor` join
  the gate as jobs, following the existing `run_tool_job` pattern. The project
  already lints its shell, its TOML, its Rust and its prose; workflow YAML that
  holds a token and runs on every push should not be the one unchecked file.
- **Automated dependency updates.** `.cargo/audit.toml` currently tolerates two
  RUSTSEC advisories with a written argument and a stated removal trigger, but
  nothing exists that would ever pull in the fix. Without update automation the
  gate simply turns red one day with no mechanism to turn it green.
- **A vulnerability disclosure channel.** `SECURITY.md`. The tool parses
  untrusted AsciiDoc and makes an explicit claim in `README.md` that source
  content can never become a rendering instruction. A project making that claim
  needs somewhere for someone to report that it is wrong.
- **Contributor paperwork.** `CONTRIBUTING.md` pointing at the gate and the
  OpenSpec workflow, `.editorconfig`, and `CODEOWNERS`.

## Capabilities

### New Capabilities

None. This change adds no requirement to `adocpdf` and removes none. Rendering
the same source with the same `--date` produces the same bytes before and after.

### Modified Capabilities

None. `.openspec.yaml` sets `skip_specs: true`.

## Non-goals

- **Publishing to crates.io.** `cargo_common_metadata` is deliberately allowed
  in `[workspace.lints]` precisely because these crates are not destined for the
  registry. Nothing here changes that.
- **Release automation.** Tagging, changelog generation and binary distribution
  belong to the `shipping-hygiene` change, not here.
- **Pushing.** `AGENTS.md` forbids `git push` and `git merge` without explicit
  consent. This change creates commits and writes the workflow; the user pushes.
  A consequence is that the workflow cannot be observed passing during apply —
  see `design.md`.
- **Branch protection, required checks, and OpenSSF Scorecard.** These are
  settings on the GitHub side of a repository that does not exist yet. The
  workflow this change adds is the prerequisite for them.
- **Reimplementing the gate as a matrix of separate CI jobs.** Tempting for
  parallelism and prettier failure reporting, and rejected in `design.md`
  because it duplicates the gate's definition.
- **Testing improvements.** Property tests, fuzzing and mutation testing are the
  `behavioural-testing` change.

## Impact

**Architectural layers: none.** No crate under `crates/` is touched, no
dependency is added to any manifest, and `architecture.toml` is unchanged. The
files affected are repository infrastructure: `.github/`, `scripts/ci/gate.sh`,
`SECURITY.md`, `CONTRIBUTING.md`, `.editorconfig`, `CODEOWNERS`, and
`AGENTS.md`/`README.md` where they document the gate's job count.

**Decisions re-opened: none.** No decision recorded in the project context is
disturbed. This change acts on `AGENTS.md`'s existing rules rather than revising
them.

**New tooling, not new dependencies.** `actionlint` and `zizmor` are developer
tools invoked by the gate, in the same category as `shellcheck` and `taplo`.
They do not appear in `Cargo.lock` and the architecture guard never sees them.

**Cost.** Two more tools to install for a full local gate run, and CI minutes on
every push. The gate's total runtime — dominated by the coverage and MSRV jobs —
becomes something contributors wait on rather than something they choose to run.
