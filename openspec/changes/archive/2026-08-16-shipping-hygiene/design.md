## Context

See `proposal.md` — Why, and its Ordering section for the dependency on
`bootstrap-repository`.

Three facts about the tree, verified this session:

- `[profile.release]` is one line, `lto = "thin"`. There is no `[profile.dev]`.
- `taiki-e/install-action` carries `cargo-auditable`, `cargo-cyclonedx` and
  `git-cliff`, but **not** `committed` — checked against its `TOOLS.md`.
- Licences of everything proposed here: `cargo-auditable` 0.7.5 and `git-cliff`
  2.13.1 are `MIT OR Apache-2.0`; `cargo-cyclonedx` 0.5.9 is `Apache-2.0`;
  `committed` 1.1.11 is `MIT OR Apache-2.0`. All satisfy `deny.toml`'s
  allow-list — relevant only if any of them ends up in the dependency graph
  rather than beside it, which D3 checks rather than assumes.

## Goals / Non-Goals

**Goals:**

- Make the artefact answerable to the same question the tree already answers:
  what is in it, and does any of it have a known vulnerability?
- Decide every field in `[profile.release]` on its merits, so that what is
  absent is absent for a reason.
- Enforce the commit convention where enforcement actually holds.

**Non-Goals:**

- Changing what `adocpdf` does. Nothing here touches `crates/`.
- Optimising for binary size as an end in itself.
- Owning the release process. See D6 on the boundary between generating a
  changelog and deciding when to release.

## Decisions

### D1 — `cargo-auditable` for release builds

`cargo auditable build --release` replaces `cargo build --release`, embedding a
compressed dependency manifest in the binary. `cargo audit bin adocpdf` then
works on a downloaded file.

The value is specific to this project rather than generic hygiene: this binary
statically links an entire layout engine and an AsciiDoc parser, and
`.cargo/audit.toml` already tolerates two advisories on the argument that they
are unreachable *in the current tree*. That argument is about a version. Without
embedded metadata there is no way to check, later, which version a given binary
actually contains.

Rejected: doing nothing and relying on release tags to identify contents. Tags
identify source, not the resolved dependency graph, and the graph is what
advisories are written against.

### D2 — `codegen-units = 1`, and `lto` stays `"thin"`

`codegen-units = 1` gives the optimiser the whole crate at once. It costs
release build time, which this project spends rarely.

`lto = "thin"` is deliberately **not** promoted to `"fat"`. Fat LTO with a
single codegen unit is the maximal setting and the obvious pairing, and there is
no measurement here to justify the extra build time. `behavioural-testing`
records that this project has no benchmark suite and why. Changing an
optimisation setting because it is the stronger-sounding option, with no
number attached, is cargo-culting. The existing `thin` was already chosen; this
change does not re-open it.

### D3 — `strip = "debuginfo"`, not `"symbols"`, and the interaction with D1 is verified

Stripping debug info removes the bulk of a release binary's size. Stripping the
symbol table as well removes a little more and makes every future backtrace a
list of hex addresses.

That matters here more than usual. `bootstrap-repository` establishes a
`SECURITY.md` stating that a panic on untrusted input *is* a vulnerability. The
first artefact of such a report is a backtrace from a user, and a stripped
symbol table turns a diagnosable report into an unusable one. So: `strip =
"debuginfo"`.

**Unverified, and checked rather than assumed:** whether `strip` removes the
custom section `cargo-auditable` writes. One tool adds data to the binary and
the other removes data from it, and the interaction is not obvious from either
tool's documentation. The check is empirical and cheap — build with both
enabled, then run `cargo audit bin` on the result — and it is a task. If the
data does not survive, `strip` loses, because auditability is the point of this
change and binary size is not.

### D4 — `panic = "abort"` is rejected

Rejected deliberately, so that its absence reads as a decision.

1. **It contradicts a requirement.** The spec says malformed source must not be
   fatal. Aborting converts any residual bug on untrusted input into the most
   fatal possible outcome, with no unwinding and no destructor running.
2. **It would make the tested configuration differ from the shipped one.** The
   test harness needs unwinding, so tests keep `panic = "unwind"` regardless.
   Shipping a panic strategy no test ever exercises is exactly the kind of
   divergence the project's gate exists to prevent.
3. **The gain is small.** Some binary size and a little speed, against those
   two costs.

### D5 — `[profile.dev.package."*"] opt-level = 2`, on trial

Dependencies compile optimised; the workspace's own crates stay at
`opt-level = 0`. Debugging into this project's code is unaffected — only the
embedded Typst engine and the parser change.

**The expected speedup is unmeasured.** The reasoning — that a heavy layout
engine recompiled and re-run unoptimised dominates test wall-clock — is
plausible and is not evidence. So the task measures a clean `cargo test
--workspace` before and after and reports both numbers, and **if the improvement
does not materialise, the setting is removed rather than kept for tidiness.**
The cost is a slower first build after every dependency change, which is real
and must be weighed against a real number, not a hoped-for one.

Two interactions checked at the same time: `cargo llvm-cov` instruments the
workspace's own crates, so optimised dependencies should not move the coverage
figure; and the MSRV job runs `cargo check`, which the profile also affects.
Both are confirmed by running the gate, not by reasoning about it.

### D6 — `committed` for the commit convention; `git-cliff` for the changelog

**`committed` 1.1.11**, chosen over two alternatives:

- **`cocogitto`** also enforces Conventional Commits, and also generates
  changelogs, manages versions and drives releases. Adopting it would duplicate
  `git-cliff` and pull release policy — a non-goal — into a change that wants
  one check. If this project later wants a single tool owning the whole
  convention-to-release pipeline, `cocogitto` is the natural consolidation, and
  that is a decision to make deliberately rather than to arrive at by accident.
- **`commitlint`** would introduce Node and a `package.json` into a repository
  that has neither, for one check.

`committed` is absent from `install-action`, so CI installs it with `cargo
install committed --locked`. That is the cost of the smaller tool, and it is
paid in one job.

**Where it runs, and why not in the gate.** `scripts/ci/gate.sh` checks the
*working tree*: it can run on uncommitted changes, and every one of its
seventeen jobs makes sense before a commit exists. A commit-message linter
checks *history*, which the gate has no view of. Adding it there would mean a
job that is meaningless in the situation the gate is normally run in.

So this is the one check in the project that lives only in CI, on pull requests,
over the range of commits the PR contains. That asymmetry is stated in
`AGENTS.md` rather than left for someone to notice. A local hook was rejected on
the usual grounds — `--no-verify` exists, and a check that can be skipped by the
person it constrains is documentation.

**`git-cliff` 2.13.1** generates `CHANGELOG.md` from the enforced history,
configured by `cliff.toml`. It runs when a release is cut, not on every commit:
regenerating a changelog per commit produces a file that conflicts on every
merge and that nobody reads.

### D7 — Determinism here means PDF bytes, not binary bytes

Worth stating explicitly, because two different things share a word.

The project's requirement is that a given document plus an injected date
produces byte-identical **PDF output**, on every run and every machine. Nothing
in this change is anywhere near that path: no crate under `crates/` is touched,
and the `Clock` port, the font embedding and the emitter are all untouched.
Embedding an audit manifest changes the *binary*, and the binary's bytes are not
what the requirement is about.

Making the **binary** byte-identical across machines — `--remap-path-prefix`,
`SOURCE_DATE_EPOCH`, a pinned build environment — is a genuine and different
goal, listed as a non-goal in the proposal. Recorded here so that a future
reader asking "doesn't this project care about reproducibility?" finds the
distinction rather than an apparent contradiction.

### D8 — The SBOM is a release artefact, not a checked-in file

`cargo-cyclonedx` runs in CI when a release is cut and the output is attached to
the release. Committing a generated SBOM would produce a file that changes on
every dependency bump, conflicts on every Dependabot merge, and is stale the
moment it is not regenerated.

It is generated, not gated: an SBOM describes, it does not check. `cargo audit`
and `cargo deny` do the checking, and both are already gate jobs.

## Risks / Trade-offs

- **`strip` may silently remove the audit data.** → D3 makes this an empirical
  check with a stated tie-breaker: auditability wins over size.

- **The dev-profile change might not help, and costs a slower first build.** →
  D5 requires before-and-after numbers and removal if the improvement is not
  real. The failure mode being guarded against is keeping a plausible-sounding
  setting nobody measured.

- **`cargo-auditable` may alter the dependency graph.** If it introduces a
  runtime dependency, `deny.toml` and the architecture guard both have standing
  to object. → Checked with `cargo tree` before and after, plus a guard run.
  Not assumed either way.

- **A commit-message check that lives only in CI is easy to forget locally.** →
  Accepted, and documented in `AGENTS.md` and `CONTRIBUTING.md` with the local
  command to run before opening a PR. The alternative, a bypassable hook, gives
  less.

- **`committed` needs `cargo install` in CI**, so that job is slower than the
  others. → One job, and the alternatives cost more (D6).

- **This change is inert without `bootstrap-repository`.** Applying it first
  produces CI files with no `.github/` to sit in and a changelog generated from
  an empty history. → Stated in the proposal's Ordering section, and the first
  task checks the precondition rather than discovering it.

## Open Questions

- Whether `lto = "fat"` is worth its build time. Answerable only with a
  benchmark, which this project does not have and has deliberately deferred.
  Recorded so the question is not lost; it changes nothing here.
- The exact `cliff.toml` grouping of commit types into changelog sections.
  Cosmetic, adjustable at any time, and it affects no decision above.
