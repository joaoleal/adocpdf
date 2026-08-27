# Contributing to adocpdf

Two rules carry most of the weight: **no code before a proposal**, and **the
gate must pass**. Everything below elaborates on those.

## The workflow: propose → apply → archive

This project uses [OpenSpec](https://openspec.dev/) for spec-driven
development. Planning artifacts live in `openspec/changes/<name>/`; the
capabilities the renderer implements today live in `openspec/specs/`.

**No project code is written before a change proposal covering it exists.** A
proposal is four files — `proposal.md` (what and why), `specs/<capability>/spec.md`
(observable behaviour, or `skip_specs: true` when behaviour genuinely does not
change), `design.md` (how, and what was rejected), and `tasks.md` (independently
verifiable steps).

The CLI is not installed; run it through `npx`:

```bash
npx @fission-ai/openspec@1.9.0 list
npx @fission-ai/openspec@1.9.0 status --change "<name>" --json
npx @fission-ai/openspec@1.9.0 validate "<name>" --strict
```

Read `openspec/config.yaml` for the project context and the per-artifact rules
rather than re-deriving them. Archived changes under
`openspec/changes/archive/` are worth reading — they record why decisions were
made, including the ones that were reversed.

## The gate

Run it before considering any change done:

```bash
scripts/ci/gate.sh
```

A green typecheck is not a passing test suite. **A job whose tool is missing
fails — it never skips**, because a check that quietly skips reports success on
a machine that verified nothing.

The gate needs `~/.cargo/bin` on `PATH`, plus tools that do not ship with the
toolchain:

```bash
cargo install cargo-llvm-cov cargo-audit cargo-deny cargo-machete \
              typos-cli taplo-cli cargo-hack --locked
cargo install zizmor committed --locked
sudo apt install shellcheck          # or your platform's package manager
rustup toolchain install 1.92 --profile minimal   # the declared MSRV
```

`actionlint` is installed separately — see its
[releases](https://github.com/rhysd/actionlint/releases) or
`go install github.com/rhysd/actionlint/cmd/actionlint@latest`.

The toolchain itself is pinned in `rust-toolchain.toml`; `rustup` fetches it
automatically, including the `llvm-tools-preview` component coverage needs.

### Things the gate will not let you do

These are in `AGENTS.md` as hard constraints, and they exist because each was
tempting at some point:

- Do not lower the 90% coverage floor to make the gate pass. Write tests.
- Do not skip a test, weaken an assertion, or delete a failing test for a green
  run. A failing test is information.
- Do not silence an advisory, a licence rejection, or a lint. Each has a
  documented escape hatch requiring a written reason — use it, or fix the cause.
- Do not add a dependency without adding it to `architecture.toml`. The guard
  will fail, and that is the point.
- **Do not write a checker.** Code quality is enforced by analyzers that already
  exist. A per-file line-limit guard was built here once and deleted, because
  clippy's `too_many_lines` already covered the concern and the bespoke version
  cost 460 lines to measure a worse proxy. The architecture guard is the sole
  exception, and only because the layer table is particular to this codebase.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/), scoped by layer
where one applies:

```
feat(domain): resolve section themes through the layout plan
fix(host): reject a symlink whose target resolves outside the root
build: pin every GitHub Action to a commit SHA
docs: ...    test: ...    refactor: ...    chore: ...
```

Explain *why* in the body. The subject says what changed; the body is where a
future reader finds out what you were weighing.

This is enforced by the gate's `commit convention` job, which lints every
commit your branch adds over the default branch. **Wrap the body at 72
columns** — the limit `committed` applies is subtler than that number
suggests, and 72 always passes.

Run the gate *after* committing and before pushing. Run it before, and the job
sees an empty range and has nothing to say: it is an early warning, not a
guarantee, and CI is what blocks a merge. To lint one range by hand:

```bash
committed <base>..HEAD --no-merge-commit
```

There is no local hook, because `--no-verify` would make it optional.

The rules are in `committed.toml`. Every value that departs from the tool's
own defaults is written down there with the reason — notably that `build`,
`ci` and `revert` are allowed types, and that subjects are lowercase after the
type, matching this project's history and the example above.

`CHANGELOG.md` is generated from that history by `git-cliff` (`cliff.toml`)
when a release is cut. Do not edit it by hand, and do not regenerate it per
commit — a changelog rewritten on every push conflicts on every merge.

Never `git push` or `git merge` without the maintainer's explicit consent.
Commit freely.

## Architecture

Clean Architecture with a strict inward dependency rule, enforced by a
checked-in guard reading `architecture.toml`:

| Crate | Ring | May depend on |
|---|---|---|
| `adocpdf-core` | entities | nothing (std only) |
| `adocpdf-domain` | use cases | core |
| `adocpdf-shared` | interface adapters | core |
| `adocpdf-adapters` | interface adapters | domain, shared |
| `adocpdf-asciidoc` | frameworks | core, domain, adapters |
| `adocpdf-typst` | frameworks | core, domain |
| `adocpdf-host` | frameworks | domain, adapters |
| `adocpdf-cli` | frameworks (delivery) | core, domain, shared, adapters, asciidoc, host, typst |
| `adocpdf-wasm` | frameworks (delivery) | core, domain, shared, typst |

Each external technology is confined to one crate in production code:
`asciidoc-parser` to `adocpdf-asciidoc`, Typst to `adocpdf-typst`.
`adocpdf-cli` is additionally permitted `typst` and `typst-layout` for the
layout tests, which read the engine's own frames; nothing under its `src/` names
them.

`architecture.toml` governs `[dependencies]` and `[dev-dependencies]` alike: a
test that reaches outward breaks the layering just as a library does — which is
also why that one permission is real rather than test-only.

Two rules are easy to break and worth repeating:

- **Content is never markup.** Everything derived from an AsciiDoc source
  reaches the output through `markup::string_literal` and nothing else.
- **Paths are judged by where they resolve, not how they are spelled.** Never
  add a filesystem call that bypasses `SandboxedPath`.

See `AGENTS.md` for the full conventions.

## Repository settings

These cannot be set from the codebase. They are all **enabled** on this
repository; the list is here so a fork knows what to turn on, and so anyone
wondering why a push was refused finds the answer.

- **Branch protection on `main`**, with the `quality gate` status check
  required. That is the name of the *job* in `.github/workflows/ci.yml`, not of
  the workflow — GitHub reports status by job name, and requiring a check named
  `gate` would wait forever for one that never reports.
- **Required pull requests** — no direct pushes to `main`. Zero approvals are
  required, deliberately: a sole maintainer cannot approve their own pull
  request, so demanding one would deadlock the repository. The gate is what
  guards the branch; the review requirement exists to force the change through
  CI, not to simulate a second person.
- **Signed commits required.** Every commit on `main` must carry a valid
  signature. Set `commit.gpgsign` and a `user.signingkey` before you start, or
  your work will not be mergeable. Commits GitHub creates itself — Dependabot's,
  and anything merged through the web interface — are signed with its own key
  and satisfy this.
- **Linear history required.** Merge commits are refused, so land work by
  squashing or rebasing. This keeps `git-cliff`'s changelog readable, since a
  merge commit carries no Conventional Commits type.
- **Force pushes and branch deletion blocked** on `main`.
- **Private vulnerability reporting** enabled, so the *Report a vulnerability*
  form referenced by `SECURITY.md` actually appears.
- **Dependabot alerts and security updates** enabled, complementing the
  scheduled version updates configured in `.github/dependabot.yml`.

Administrators are not exempt from the status check by configuration, but
`enforce_admins` is off, so a maintainer retains a manual override for an
emergency. Using it should be rare enough to be worth explaining afterwards.

## Reporting a vulnerability

See `SECURITY.md`. Do not open a public issue for one.
