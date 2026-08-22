# AGENTS.md

Context for AI agents working in this repository. This is the **sole** agent
context file.

## Hard constraints

- **MUST NOT** create `CLAUDE.md`. This project uses `AGENTS.md` as its only
  agent context file.
- **MUST NOT** `git push` or `git merge` without explicit user consent. Commit
  freely; ask before pushing or merging.
- **MUST NOT** skip tests, weaken an assertion, or delete a failing test to get
  a green run. A failing test is information.
- **MUST NOT** write project code before an OpenSpec change proposal covering it
  exists. See *Workflow* below.
- **MUST NOT** add a dependency without adding it to `architecture.toml`. The
  guard will fail, and that is the point.
- **MUST NOT** lower the coverage floor to make the gate pass. It is 90% line
  coverage, workspace-wide, in `scripts/ci/gate.sh`. If coverage drops, write
  tests.
- **MUST NOT** silence an advisory, a licence rejection, or a lint to get a
  green run. Each has a documented escape hatch that requires a written reason
  — use it, or fix the cause.
- **MUST NOT** add a coverage-exclusion attribute to code that could be tested
  with a fake. Exclusions are for genuinely unreachable code, with a comment
  saying why.

## Workflow

This project uses **OpenSpec** (`@fission-ai/openspec`), not Speckit. The order
is **propose → apply → archive**:

```bash
npx @fission-ai/openspec@1.9.0 status --change "<name>" --json
npx @fission-ai/openspec@1.9.0 validate <name> --strict
```

Planning artifacts live in `openspec/changes/<name>/`: `proposal.md`,
`specs/<capability>/spec.md`, `design.md`, `tasks.md`. `openspec/config.yaml`
carries the project context and the per-artifact rules; read it rather than
re-deriving the conventions below.

`/opsx:propose` is planning-only — it must not edit project code, even when the
request that triggered it asks to build something.

## Architecture

Clean Architecture with a strict inward dependency rule, enforced by a checked-in
guard (`xtask/tests/architecture.rs`) reading `architecture.toml`:

| Crate | May depend on |
|---|---|
| `adocpdf-core` | nothing (std only) |
| `adocpdf-domain` | core |
| `adocpdf-shared` | core |
| `adocpdf-infra` | core, domain, shared |
| `adocpdf-cli` | core, domain, shared, infra |
| `adocpdf-wasm` | core, domain, shared, infra |

- `adocpdf-core` — document and theme model. Zero dependencies, so its errors
  carry hand-written `Display`/`Error` impls rather than using `thiserror`.
- `adocpdf-domain` — entities, value objects, ports, use cases. Must never name
  Typst, the AsciiDoc parser, the filesystem, or a delivery mechanism.
- `adocpdf-shared` — boundary DTOs only, free of business rules. The mapping
  between DTOs and domain types lives in `adocpdf-infra`, because that is the
  innermost layer that can see both.
- `adocpdf-infra` — adapters implementing domain ports. The only layer naming an
  external technology. Maps foreign errors to `DomainError` at the boundary.
- `adocpdf-cli` / `adocpdf-wasm` — composition roots. No business logic.
- `xtask` — tooling. Not a layer; nothing may depend on it.

## Conventions

- Fallible domain operations return `Result<T, DomainError>`. Never a raw
  string, `anyhow`, or a bare `Box<dyn Error>` in domain or application code.
- Value objects validate on construction and are immutable after. An invalid
  value must not be representable.
- Ports are traits owned by the domain; adapters live in infra and are injected
  at a composition root. No DI framework, no global singletons.
- Domain use cases are tested with hand-written in-memory fakes, not a mocking
  crate. Adapters get integration tests against real I/O.
- Test names describe behaviour. Never name a test after a task or requirement
  id.
- No `unsafe` (forbidden workspace-wide). No `unwrap`/`expect` in production
  paths — permitted in tests, and for provably unreachable invariants with an
  `#[allow(..., reason = "...")]` saying why.
- Public items carry rustdoc; every public fallible function documents its
  `# Errors`. Comments explain *why*, not *what*.
- Determinism: identical inputs produce byte-identical output. Nothing may
  depend on wall-clock time, locale, ambient machine state, or map iteration
  order. The date is injected through the `Clock` port.
- Conventional Commits (`feat(domain): ...`).

## Two rules that are easy to break

**Content is never markup.** Everything derived from an AsciiDoc source reaches
the output through `markup::string_literal` and nothing else. Structural
instructions are built only from validated model values. Adding a new emitter
path that interpolates source text directly is an injection.

**Structure is spelled in an alphabet the document cannot type.** Inline
structure crosses from the parser to the model as characters in a string, and
the characters it uses are Unicode noncharacters (U+FDD0–U+FDEF), which
`parser::refuse_input_that_could_forge_structure` refuses in source. That is
what makes a forged span impossible rather than merely escaped — and it is load
bearing, because `pass:[…]` and `+++…+++` reach the parser's output verbatim,
without substitution. Never introduce an in-band encoding whose alphabet a
document can type, and never relax the guard.

Relatedly: special characters must be rendered as `&lt;`, `&gt;` and `&amp;`
and no other spelling. Later substitution steps in `asciidoc-parser` match
those exact strings — the arrow replacements are `Regex::new(r"\\?-&gt;")` —
so a different encoding silently stops them firing.

**Paths are judged by where they resolve, not how they are spelled.** The
containment rule lives in `adocpdf-domain::sandbox`; resolution is the
`PathResolver` port. Never add a filesystem call that bypasses `SandboxedPath`.

## The gate

```bash
scripts/ci/gate.sh
```

Eighteen jobs: formatting, lints, tests, the architecture guard, the WASM
build, docs, MSRV, shell, TOML, spelling, feature combinations, workflow syntax,
workflow security, commit convention, advisories, licences, unused
dependencies, and coverage. A green typecheck is not a passing test suite. Run
it before considering a change done.

Requires `~/.cargo/bin` on `PATH`, plus the tools that do not ship with the
toolchain:

```bash
cargo install cargo-llvm-cov cargo-audit cargo-deny cargo-machete \
              typos-cli taplo-cli cargo-hack zizmor committed --locked
sudo apt install shellcheck
rustup toolchain install 1.92 --profile minimal
```

`actionlint` is not on crates.io; install its release binary from
<https://github.com/rhysd/actionlint/releases>.

A job whose tool is missing **fails**; it never skips.

### Three instruments that are not in the gate

The gate answers "does this change break anything we already check". These
three answer different questions, take too long for a merge to wait on, and run
on a schedule instead. None of them is a substitute for a test.

| Instrument | Question it answers | Where it runs |
|---|---|---|
| `proptest` | does the rule hold for *every* input, not just the examples | in the gate, as ordinary tests |
| `cargo-fuzz` | is there an input nobody thought of that panics or hangs | `.github/workflows/fuzz.yml`, weekly, **nightly toolchain** |
| `cargo-mutants` | would any test have noticed if this code were wrong | `.github/workflows/mutants.yml`, weekly |

**Property tests are the exception — they are in the gate**, because they are
ordinary `#[test]`s. `crates/adocpdf-infra/src/markup.rs`,
`crates/adocpdf-infra/src/inline.rs` and `crates/adocpdf-domain/src/sandbox.rs`
each carry a `properties` module, and `crates/adocpdf-infra/tests/injection.rs`
runs the same claims through the real engine at a lower case count. Failing seeds are saved to
`proptest-regressions/` and **are committed** — that directory is a record of
every counterexample ever found, and deleting it throws that away.

**Fuzzing needs nightly** and cannot run from the pinned toolchain:
`cargo-fuzz` depends on LLVM sanitizer instrumentation, which is unstable.

```bash
rustup toolchain install nightly
cargo install cargo-fuzz --locked
cargo +nightly fuzz run parse_plan_emit -- -max_total_time=300 -timeout=10
```

Pass `-timeout=N`. libFuzzer's default per-input timeout is 1200 seconds, so a
hang looks identical to slow progress until twenty minutes have passed — which
is exactly how the `U+000C` defect nearly went unnoticed.

`fuzz/` is **not** part of the workspace, and the root manifest excludes it
explicitly. `libfuzzer-sys` is `(MIT OR Apache-2.0) AND NCSA`, a conjunction,
and NCSA is not in `deny.toml`'s allow-list — see the note in that file.

**What none of this proves.** The gate does not cover fuzzing or mutation
testing. A green gate means the checks in `scripts/ci/gate.sh` passed; it says
nothing about whether the fuzzer has found something since, and mutation
testing is enforced on three files only — the injection boundary, the inline
decoder and the sandbox rule — with the rest of the workspace reported as
information and no threshold applied.

### Commit messages, and a decision this project reversed

`committed` enforces Conventional Commits, and it is the gate's `commit
convention` job. It lints every commit the current branch adds over the
default branch, resolving that branch from `origin/HEAD`, then `origin/main`,
then `main`, and failing rather than passing if none of the three resolves.

It was not always there, and the reasoning that kept it out is worth keeping
on the page.

**What was decided.** The gate checks the **working tree**: it can run on
uncommitted changes, and every one of its jobs made sense before a commit
existed. A commit-message linter checks **history**, which the gate had no
view of. A job that is meaningless in the situation the gate is normally run
in did not belong in it, so the check lived in its own workflow.

**Why that was reversed.** The premise was right and the conclusion was not.
Running the gate before any commit exists gives the job an empty commit
range, and an empty range is a *check of zero commits*, not a skip — nothing
in front of it is left unverified. That is the distinction this gate cares
about: it objects to a check that quietly skips because such a check reports
success on a machine that verified nothing, which is not what happens here.

The cost of the old arrangement was paid in full on pull request #4. The gate
went green — all seventeen jobs, 505 tests, 95.52% line coverage — while the
separate workflow failed on a commit body wrapped at 77 characters. Nothing a
contributor could run locally would have said so. Worse, that workflow was
never a required status check: `quality gate` is the only one, so the rule
this file mandates could not actually block a merge. Folding the check into
the gate made it blocking without touching branch protection.

**What this still does not close.** A gate run *before* you write a commit
cannot see the message you write after it. The gate is an early warning, run
again after committing and before pushing; CI is the enforcement.

**What was not reversed.** A local hook is still rejected — `--no-verify`
exists, so a check the author can skip is documentation. The gate is not a
hook: nothing wires it into `git commit`, and it has no bypass flag.

To lint one range by hand:

```bash
committed <base>..HEAD --no-merge-commit
```

`cargo install committed --locked` — `taiki-e/install-action` does not carry
it. Its rules live in `committed.toml`, and each departure from the tool's
defaults is recorded there with a reason. Wrap a body at 72 columns: the
limit `committed` applies is subtler than the number suggests, and 72 always
passes.

`CHANGELOG.md` is generated from that history by `git-cliff`, configured in
`cliff.toml`. It is regenerated **when a release is cut**, not per commit: a
changelog rewritten on every push conflicts on every merge. No CI job
regenerates it, and it is not edited by hand.

### Where each rule is configured

| Rule | File |
|---|---|
| Lint sets, and every workspace-level exception | `[workspace.lints]` in `Cargo.toml` |
| Function length, cognitive complexity, doc idents | `clippy.toml` |
| Formatting policy | `rustfmt.toml` |
| TOML layout | `taplo.toml` |
| Spelling allow-list | `_typos.toml` |
| Dependency direction, per-layer allow-lists | `architecture.toml` |
| Licence allow-list, bans, sources | `deny.toml` |
| Tolerated advisories, with reasons | `.cargo/audit.toml` |
| Coverage floor, MSRV | `MIN_LINE_COVERAGE`, `MSRV` in `scripts/ci/gate.sh` |
| Editor defaults, mirroring the formatters | `.editorconfig` |
| Commit-message rules, and every departure from the tool's defaults | `committed.toml` |
| Changelog grouping and templates | `cliff.toml` |
| Mutation-testing exclusions, each with a reason | `.cargo/mutants.toml` |
| Fuzz targets, and the crate deliberately outside the workspace | `fuzz/` |
| What CI runs, and the SHA every action is pinned to | `.github/workflows/ci.yml` |
| Dependency update schedule | `.github/dependabot.yml` |

### Do not write a checker

Code quality is enforced by tools that already exist. Before adding any custom
check, establish that no analyzer does it — a per-file line-limit guard was
built here once and deleted, because clippy's `too_many_lines` already covered
the real concern and the bespoke version cost 460 lines to measure a worse
proxy.

The architecture guard is the sole exception, and only because the layer table
is particular to this codebase. The test is whether the rule is *general* — in
which case a tool implements it — or *particular to this project*.

### Exceptions need reasons

A lint that is genuinely wrong here is disabled **once**, in
`[workspace.lints]`, with a comment saying why. Never scatter `#[allow]`
attributes: an exemption nobody can find is not a decision, it is a leak. The
same applies to tolerated advisories in `.cargo/audit.toml` and to any coverage
exclusion.
