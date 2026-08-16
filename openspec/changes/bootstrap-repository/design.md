## Context

See `proposal.md` — Why. Three constraints shape everything below.

**The gate already exists.** `scripts/ci/gate.sh` is 160 lines defining fifteen
jobs through two helpers, `run_job` and `run_tool_job`. It has a stated
principle — *a job whose tool is missing FAILS, it never skips* — and a
structural property worth noticing: it collects failures into a `failed=()`
array and keeps going, so one run reports every failing job rather than the
first. CI's job is to run this, not to become a second copy of it.

**Nothing is committed.** Verified this session: `git log` reports *"your current
branch 'main' does not have any commits yet"*, and `git ls-files` returns zero.
There is no history to append to and no baseline to diff against. The first
commit is a decision about how to present ~7,300 lines that arrived at once.

**Apply cannot reach the finish line.** `AGENTS.md` forbids `git push` without
explicit consent, and a GitHub Actions workflow is unobservable until pushed.
This is addressed in D8 rather than worked around.

## Goals / Non-Goals

**Goals:**

- One definition of the quality gate, executed identically by a contributor and
  by the merge check.
- A CI run that fails for the same reasons, with the same messages, as
  `scripts/ci/gate.sh` on a laptop.
- The workflow file held to the same standard as every other file in the
  repository: linted, pinned, least-privilege.
- A history in which `git blame` on any line lands on a commit that explains it.

**Non-Goals:**

- **Cross-platform CI.** One runner, `ubuntu-latest`. The gate is bash, and the
  MSRV, coverage and WASM claims are made about Linux. Extending to macOS and
  Windows is real work with real findings and belongs to its own change. The
  consequence — those claims are unverified on other platforms — is stated in
  `README.md` rather than quietly assumed away.
- **Job-level parallelism in CI.** See D1.
- **Repository settings.** Branch protection, required status checks and private
  vulnerability reporting are configured in GitHub's UI on a repository that
  does not yet exist. `CONTRIBUTING.md` records which settings the workflow
  expects; nothing here can apply them.

## Decisions

### D1 — CI invokes `scripts/ci/gate.sh`; it does not reimplement the jobs

The workflow's check step is one line: `scripts/ci/gate.sh`.

The obvious alternative is a job matrix — a `fmt` job, a `clippy` job, a
`coverage` job — running in parallel with a tidy per-check status on the PR.
Rejected, for three reasons:

1. **Two definitions drift.** The moment a job exists in YAML and in bash, the
   local gate and the merge check answer different questions, and the difference
   surfaces as "it passes on my machine" — the exact failure mode a gate exists
   to prevent.
2. **The missing-tool principle would have to be re-encoded.** On a GitHub
   runner a tool is missing because an install step was forgotten. A matrix job
   would silently not run it; the gate fails loudly. Reimplementing that
   discipline per-job in YAML is how it gets lost.
3. **Reproduction becomes translation.** A contributor seeing a red check runs
   the same command locally and gets the same output.

The cost is real: the jobs run serially in one long job, and the PR shows one
red check rather than pinpointing which. It is partly offset by the gate's
accumulate-and-continue behaviour — a single run lists every failing job by name
at the end, where a fail-fast matrix would stop at the first. A matrix that
fails fast tells you less than one serial run that finishes.

### D2 — Gate tools are installed as prebuilt binaries via `taiki-e/install-action`

`cargo install cargo-deny cargo-llvm-cov cargo-audit cargo-machete typos-cli
taplo-cli cargo-hack --locked` compiles seven substantial tools from source on
every cache miss.

**Verified** against `taiki-e/install-action`'s `TOOLS.md`: `cargo-llvm-cov`,
`cargo-audit`, `cargo-deny`, `cargo-machete`, `cargo-hack`, `typos`, `taplo`,
`shellcheck` and `zizmor` are all supported, installed from upstream release
binaries. **`actionlint` is not supported by that action** — it needs its own
install path (its own release binaries, or the maintainer's install script).
This asymmetry is the reason D3 treats the two linters separately.

`shellcheck` is on that list too, so CI does not need `apt`.

Rejected: `cargo-binstall`, which does the same job but resolves versions at run
time from third-party sources. `install-action` pins tool versions to the
action's own release, which is itself SHA-pinned — one fewer moving part in a
change whose theme is that nothing unpinned should run.

### D3 — The workflow files are linted, by two tools with different jobs

- **`actionlint`** — correctness. Expression syntax, invalid `runs-on` values,
  and `shellcheck` over embedded `run:` blocks, which are otherwise the one
  patch of shell in this repository the gate does not check.
- **`zizmor`** — security. Script injection through untrusted interpolation,
  dangerous triggers, excessive permissions, unpinned actions.

They overlap barely; using one is not a reason to skip the other. Both are added
to `scripts/ci/gate.sh` via the existing `run_tool_job`, so a contributor
editing a workflow gets the same failure locally, and both are subject to the
missing-tool-fails rule.

Findings from either tool are fixed, not suppressed — `AGENTS.md`'s "Exceptions
need reasons" rule applies to workflow linters exactly as it does to clippy.

### D4 — Actions are pinned by commit SHA, and permissions are least-privilege

Every `uses:` names a full 40-character commit SHA with the human-readable
version in a trailing comment. A tag is a mutable pointer; a compromised action
repository can retarget `@v4` at new code, which is the shape of most recent
Actions supply-chain incidents. `zizmor` enforces this, so the rule is checked
rather than remembered.

The workflow declares `permissions: contents: read` at the top level. The gate
reads the checkout and writes nothing back; no job needs more. Anything that
later needs a wider grant asks for it on that job alone.

SHA pins go stale, which is why D5's Dependabot config covers the
`github-actions` ecosystem and not only `cargo`. Pinning without an update
mechanism is how a repository ends up running a two-year-old action.

### D5 — Dependabot, not Renovate

Renovate is the more capable tool: dependency grouping, update cooldowns,
custom managers, far better control over PR volume. It also requires installing
a GitHub App and maintaining a `renovate.json` that becomes its own artifact.

Dependabot is native, needs no installation, and is configured by one small
YAML file. This workspace has six direct third-party dependencies declared in
`[workspace.dependencies]`. Choosing the tool with more machinery than the
problem needs would contradict the rule that produced `AGENTS.md`'s "Do not
write a checker" section.

Configuration: the `cargo` and `github-actions` ecosystems, weekly, against the
default branch.

**Unverified:** whether Dependabot edits `[workspace.dependencies]` entries
correctly when member crates inherit them via `foo.workspace = true`. Cargo
workspace support exists; correct handling of *inherited* dependencies in this
exact layout was not confirmed. This is the first thing to check after the
initial run, and the trigger for revisiting Renovate if it turns out to be
wrong. It is recorded as a task, not assumed.

A Dependabot PR that bumps a crate into a licence outside `deny.toml`'s
allow-list, or into a new advisory, will fail the gate. That is the gate
working, not a defect.

### D6 — `Cargo.lock` is tracked

The prevailing template advice — *libraries do not commit `Cargo.lock`* — does
not apply. This workspace's product is `adocpdf`, a binary. Three project
commitments depend on the lockfile being in the repository:

- **Determinism.** `AGENTS.md` requires identical inputs to produce
  byte-identical output. An untracked lockfile means two checkouts can resolve
  different dependency versions and produce different PDFs from the same source.
- **`cargo audit`** reads `Cargo.lock`. Auditing a tree whose lockfile is
  regenerated per machine audits whatever resolved this morning.
- **`deny.toml` sets `wildcards = "deny"`** for exactly this reason — the
  reproducibility argument is already made in that file, and leaving the
  lockfile untracked contradicts it.

### D7 — The initial history is grouped by OpenSpec change, and says that it is

Two bad options: one `chore: initial commit` containing everything, which makes
`git blame` useless forever; or a hand-crafted sequence of commits pretending to
be the development that actually happened, which is fabrication.

The chosen middle: commits grouped by the units the project already organises
work into — `render-first-pdf`, then `quality-gates`, then this change — since
those are genuinely separable and each has a proposal explaining it. The first
commit's body states plainly that the history is a reconstruction of an existing
working tree, not a replay. A reader who later wonders why 7,000 lines appear in
three commits finds the answer in the commit itself.

Conventional Commits throughout, per `AGENTS.md`.

### D8 — The workflow is verified statically; its first real run is the user's

Apply can prove: `actionlint` and `zizmor` pass on the workflow, `scripts/ci/
gate.sh` passes locally including the two new jobs, and the referenced action
SHAs resolve to the claimed versions. Apply cannot prove the workflow runs
green on a GitHub runner, because that requires a push, and pushing requires
consent this change does not have.

Running the workflow locally under `act` was considered and rejected: it needs
Docker, it emulates the runner imperfectly, and a green `act` run would be
weaker evidence presented as stronger.

So the tasks end with an explicit handover: the workflow is *unverified in
execution* until the user pushes, and the task list says so rather than claiming
a completion that did not happen. Likely first-run failures — a missing
toolchain component, a `PATH` assumption — are cheap to fix once observed and
expensive to guess at now.

**Outcome, recorded during apply.** This constraint was lifted: the user gave
explicit consent to create `github.com/joaoleal/adocpdf` and push, so the
workflow did execute. Run `31953888060` succeeded on the first attempt — every
provisioning step and all seventeen gate jobs, ending in `gate passed`. Two of
the guesses this decision worried about turned out to matter and had already
been fixed before pushing: `actionlint` installs to `/usr/local/bin` rather than
`$HOME/.local/bin`, because whether the latter is on `PATH` is a property of the
runner image; and the MSRV is read out of `gate.sh` instead of duplicated in the
workflow. The handover in task 7.2 is therefore a real result rather than a list
of unproven claims.

### D9 — `SECURITY.md` states the security model, not just an address

A disclosure file that says only "email us" leaves a reporter guessing what
counts as a vulnerability. This one states the boundaries the project actually
claims, each of which is already documented and tested:

- Source content reaches output only through `markup::string_literal`; anything
  that makes AsciiDoc content act as a Typst instruction is a vulnerability.
- Reads and writes are confined to the project root, judged by where a path
  resolves; any escape is a vulnerability.
- A panic or non-termination on malformed input is a vulnerability, because the
  input is untrusted by definition.
- Vulnerabilities in the embedded Typst engine or in `asciidoc-parser` are
  reported upstream; this project tracks them through `cargo audit`.

Reporting channel: GitHub private vulnerability reporting, with the caveat that
it must be enabled in repository settings — recorded in `CONTRIBUTING.md`
alongside the other settings this change cannot apply.

### D10 — Documentation that counts jobs must be recounted

`README.md` and `AGENTS.md` both say the gate has *fifteen* jobs, and
`README.md` carries a table with one row per job. Adding `actionlint` and
`zizmor` makes it seventeen. Both files are updated in the same task that adds
the jobs, so the count is never briefly wrong — the project's own rule that a
check must not report success on something it did not verify applies to its
prose too.

## Risks / Trade-offs

- **CI wall-clock time.** `coverage`, `msrv` and `feature combinations` each
  rebuild the workspace, and the tree embeds Typst. A full gate run is likely
  several minutes even warm. → `Swatinem/rust-cache` keyed on the toolchain
  version; the serial-job cost is accepted per D1. If runs become intolerable,
  the fix is moving the slowest jobs to a scheduled run, not deleting them.

- **A missing tool in CI is a hard failure.** By design, and it means a tool
  dropped from `install-action`'s manifest breaks every build until the install
  step is fixed. → Correct behaviour, and the alternative is a green build that
  checked less than it claimed.

- **`zizmor` may flag the workflow this change writes.** → Fixed, not
  suppressed. If a finding is genuinely wrong here it is disabled once with a
  written reason, exactly as `[workspace.lints]` handles a wrong clippy lint.

- **Dependabot PR volume against a strict gate.** Weekly bumps that each trigger
  a full gate run, some of which will fail on the licence allow-list or a new
  advisory. → Weekly rather than daily. Failures are information; the
  alternative is discovering the same breakage months later with more changes
  in between.

- **Linux-only verification.** The MSRV, coverage and WASM-build claims hold on
  `ubuntu-latest` and are unverified elsewhere. → Stated in `README.md` rather
  than implied. Widening the matrix is a separate change with a real chance of
  finding real bugs.

- **The reconstructed history is not real history.** → D7's first commit says
  so. The alternative, silence, is worse.

- **No boundary is touched.** This change adds no code to any crate under
  `crates/`, no dependency to any manifest, and no path to
  `architecture.toml`. The injection boundary (`markup::string_literal`), the
  sandbox (`adocpdf-domain::sandbox`) and the determinism boundary (the `Clock`
  port) are all untouched — with the exception that committing `Cargo.lock`
  strengthens the determinism guarantee, per D6.

## Open Questions

- Whether the gate's total CI runtime justifies moving `feature combinations`
  and `msrv` to a scheduled daily run rather than every push. Answerable only by
  measuring a real run, and changing it later touches one YAML file and no
  design decision here.
- Whether `CODEOWNERS` should name a team rather than an individual. Depends on
  whether the repository gains other contributors; the file is one line to
  change either way.
