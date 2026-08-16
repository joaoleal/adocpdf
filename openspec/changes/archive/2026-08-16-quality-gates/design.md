## Context

See proposal.md — Why. The gate today (`scripts/ci/gate.sh`) runs five jobs
through a `run_job` helper that records failures and reports them together at
the end, so one failing job does not hide the others. That structure is kept;
this change adds jobs to it.

One measurement taken before designing, because it contradicts the obvious
approach:

**The dependency tree carries more than twenty distinct licence expressions.**
The common ones are `MIT OR Apache-2.0` (164 crates), `MIT` (51) and
`Apache-2.0` (25), but the tail includes `Unicode-3.0` (27), `Zlib`,
`BSD-2-Clause`, `BSD-3-Clause`, `Unlicense OR MIT`, `0BSD`,
`Apache-2.0 WITH LLVM-exception`, and one `(Apache-2.0 OR MIT) AND
BSD-3-Clause`. An allow-list written from memory would have failed on the first
run. [verified from `cargo metadata`]

## Goals / Non-Goals

**Goals:**

- Make the licence obligation, the coverage floor and the dependency hygiene
  rules *mechanical*, so they cannot regress quietly.
- Keep every new check reportable: a failure must say what to do about it.
- Reach for an existing analyzer before writing anything. Custom code is
  reserved for rules that are particular to this codebase and that no tool can
  know — the layer table is the only one that qualifies.

**Non-Goals (design-level; see proposal.md for scope non-goals):**

- No abstraction over the checking tools. Each is invoked directly. Wrapping
  three unrelated tools behind a common interface would buy nothing.
- No attempt to make coverage measurement work without `cargo-llvm-cov`. There
  is no acceptable fallback: a coverage number produced by a different mechanism
  is not comparable to the threshold.

## Decisions

### D1. Code quality is enforced by established analyzers, not by bespoke guards

**Decision.** Every code-quality rule comes from a tool that already exists.
No custom checker is written for this change.

**Why this replaces an earlier decision.** The first version of D1 specified a
max-lines-per-file guard, implemented in `xtask` on the same data-driven pattern
as the architecture guard. It was built and it worked: 196 lines of
implementation, 264 of tests, an override table with enforced reasons, and
stale-override detection.

It was still the wrong thing to build. Clippy's `too_many_lines` already caps
function length, which is the concern that actually matters — a 400-line file of
short, clear functions is fine, and a 200-line file containing one function is
not. The custom guard measured the weaker proxy, and cost 460 lines of code to
do it. Worse, it crowded out the analyzers below, which catch far more and cost
nothing to maintain.

**What replaces it.** Configuration of tools that exist:

| Concern | Tool | Config |
|---|---|---|
| Function length, complexity, idiom, correctness | clippy — `pedantic`, `nursery`, `cargo`, selected `restriction` | `[workspace.lints]`, `clippy.toml` |
| Language idiom, dead abstractions, casts | rustc lint groups | `[workspace.lints.rust]` |
| Formatting policy | rustfmt, explicitly configured | `rustfmt.toml` |
| Shell correctness | shellcheck | — |
| TOML validity and layout | taplo | `taplo.toml` |
| Spelling in code and docs | typos | `_typos.toml` |
| Every feature combination compiles | cargo-hack | — |
| The declared MSRV is real | cargo-msrv | — |

**The architecture guard stays.** It is custom for a reason no tool can supply:
the layer table is specific to this project, and nothing off the shelf knows
that `adocpdf-domain` may not name Typst. The distinction is whether the rule is
*general* — in which case a tool already implements it — or *particular to this
codebase*, in which case there is nothing to reuse.

**Cost.** Four more tools to install, and a stricter lint set that will surface
findings in existing code. Both are one-off.

### D2. A missing tool fails the gate; it never skips

**Decision.** Each new job checks its tool is present and fails with an install
command when it is not.

**Why.** A check that skips when its tool is absent reports success on a machine
that verified nothing. That is worse than not having the check, because it is
indistinguishable from a real pass. The failure message carries the exact
`cargo install` line so the fix takes one paste.

**Cost.** A fresh clone cannot run the full gate until three tools are
installed. That is the honest cost of the guarantee, and it is documented in
the README rather than worked around.

### D2a. Advisories are `cargo-audit`'s job; licences are `cargo-deny`'s

**Decision.** `cargo-audit` checks `Cargo.lock` against the RUSTSEC advisory
database. `cargo-deny` is configured for licences, bans and duplicates only, and
its advisories check stays off.

**Why two tools when `cargo-deny` can do both.** Overlapping checks are worse
than either alone: two tools reading the same database and disagreeing about
severity produces an argument about which to believe, and silencing an advisory
then has to be done twice, in two syntaxes. Giving each tool one job means a
failure names its own cause.

`cargo-audit` is the RUSTSEC project's own front end, so its behaviour tracks
the database it reads. `cargo-deny`'s strength is policy over the dependency
graph — which licences are acceptable, which crates are forbidden, which
duplicates are tolerated — and that is what it is used for here.

**Alternative rejected.** `cargo-deny check advisories` alone, dropping
`cargo-audit`. Fewer tools to install, and it was the original proposal. It is
not what was asked for, and the separation above is a better arrangement on its
own merits.

**Consequence.** `deny.toml` carries no `[advisories]` policy. If one is added
later, D2a is the decision being reversed and should be revisited explicitly
rather than drifting.

### D3. The licence allow-list is explicit, and derived from the actual tree

**Decision.** `deny.toml` lists permitted licence identifiers explicitly. Any
dependency whose licence is not on the list fails the build, and adding one is a
deliberate edit.

**Why.** This is the mechanical form of the obligation `LICENSING.md` already
describes. The point is not to discover licences — that is already documented —
but to make a *new* dependency with a copyleft licence impossible to merge
without someone noticing.

**Composition.** The list is built from the surveyed tree above, not from
memory: MIT, Apache-2.0 (with the LLVM exception), BSD-2-Clause, BSD-3-Clause,
ISC, Zlib, Unicode-3.0, Unlicense, 0BSD, and CC0-1.0. Strong copyleft — GPL,
AGPL, LGPL, MPL — is absent by construction, which is the property that matters.

**Note.** Permitting Apache-2.0 does not weaken anything: the project is already
Apache-2.0 because the engine forces it. The allow-list prevents the tree
acquiring something *stricter*, not something equally permissive.

### D4. Coverage is line coverage, whole workspace, 90%

**Decision.** `cargo llvm-cov --workspace --fail-under-lines 90`.

**Why line coverage.** It is the measure the threshold was chosen against, and
it is the one whose meaning is unambiguous. Region coverage would be a stronger
check, but 90% region is a substantially higher bar than 90% line — adopting it
under the same number would be quietly changing the requirement.

**Why the whole workspace.** Excluding the delivery crates was considered and
rejected. `adocpdf-cli` contains the composition root *and* argument parsing,
exit-code mapping and date parsing — all of which are testable and already
tested. Excluding the crate to avoid the handful of lines in `main` would also
excuse the rest.

**What happens to genuinely uncoverable lines.** `main` itself, and any arm
that cannot be reached without a failing allocation, are excluded by attribute
at the point where they occur, with a comment saying why. That keeps the
exclusion visible in the code rather than hidden in a tool configuration file
where nobody reads it.

**Unverified.** Current coverage is unknown — nothing has measured it. The 90%
floor may require substantial new tests, and the work of reaching it cannot be
estimated until the first measurement runs. This is why measuring comes before
enforcing in the task order.

### D5. Clippy gains complexity limits, not more lint groups

**Decision.** Clippy remains a mandatory gate job, unchanged in invocation:
`cargo clippy --workspace --all-targets -- -D warnings`. What changes is its
configuration — `clippy.toml` gains `too-many-lines-threshold` and
`cognitive-complexity-threshold`, and `[workspace.lints]` adds the `nursery` and
`cargo` groups alongside `pedantic`.

**Why.** These are the thresholds behind `too_many_lines` and
`cognitive_complexity`, which are the off-the-shelf answer to "is this function
doing too much" — the concern the withdrawn file-length guard was reaching for.

`nursery` and `cargo` are now adopted too, reversing this decision's earlier
position. The objection was toolchain churn; the answer is that
`rust-toolchain.toml` pins the compiler, so a nursery lint cannot change
behaviour without a deliberate upgrade.

### D6. Third-party tools, verified

Versions and licences confirmed against the crates.io API on 2026-08-16. None
of these enter the dependency tree — they are developer tools invoked by the
gate:

| Tool | Version | Licence | Job |
|---|---|---|---|
| `cargo-llvm-cov` | 0.8.7 | Apache-2.0 OR MIT | coverage |
| `cargo-audit` | 0.22.2 | Apache-2.0 OR MIT | advisories |
| `cargo-deny` | 0.20.2 | MIT OR Apache-2.0 | licences, bans, sources |
| `cargo-machete` | 0.9.2 | MIT | unused dependencies |
| `typos-cli` | see task 2.1 | MIT OR Apache-2.0 | spelling |
| `taplo-cli` | see task 2.1 | MIT | TOML lint and format |
| `cargo-hack` | see task 2.1 | MIT OR Apache-2.0 | feature combinations |
| `shellcheck` | distribution package | GPL-3.0 | shell correctness |

`shellcheck` is GPL-3.0, which the licence allow-list forbids. That is not a
conflict: it is a distribution package invoked as a program, never linked into
anything this project ships, so it never enters the dependency tree
`cargo-deny` inspects. The distinction is between *using* a tool and
*distributing* its code.

Versions for the tools added by this revision are recorded during
implementation rather than guessed at here.

`clippy` and `rustfmt` are not listed: they ship with the pinned toolchain
(`rust-toolchain.toml` names both as components), so there is nothing to install
and no version to track separately.

All three clear the pinned toolchain (1.97.1). Because they are not
dependencies, they do not appear in `architecture.toml` and cannot affect the
layer rules.

## Risks / Trade-offs

- **Coverage may be far below 90%.** → Measure first, then decide. The task
  order puts measurement before enforcement so the gap is known before the
  threshold is switched on. If the gap is large, the honest response is to
  report it rather than to quietly lower the floor.

- **A 90% floor can be met with assertion-free tests.** → Not mitigated by this
  change; mutation testing is the real answer and is a stated non-goal. Worth
  saying out loud so the number is not mistaken for more than it measures.

- **The advisory check needs network access.** → Accepted. `cargo-audit` fails
  loudly when it cannot fetch the RUSTSEC database, per D2. Vendoring the
  database is a CI concern.

- **A new advisory can fail the gate without any code changing.** → That is the
  check working. The trade-off is that the build's success depends on the
  outside world; the alternative is not knowing.

- **`clippy::nursery` lints are unstable and may churn on toolchain upgrades.**
  → Accepted deliberately, reversing an earlier position. The churn is a pinned
  toolchain away from being a problem: `rust-toolchain.toml` fixes the compiler,
  so a nursery lint changes behaviour only when someone chooses to upgrade. The
  earlier decision traded real findings for a hypothetical maintenance cost.

- **A stricter lint set may surface many findings at once.** → They are fixed,
  not allowed. If a lint proves genuinely wrong for this codebase it is
  disabled in `[workspace.lints]` with a comment saying why — one visible
  decision, not scattered `#[allow]` attributes.

- **Three more tools to install.** → Documented in the README, and the failure
  message names the install command.

## Open Questions

- **Should the floor become a ratchet later?** A never-decreasing high-water
  mark catches slow erosion that a fixed floor permits. Worth revisiting once
  the fixed floor has held for a few changes; deciding now would be guessing.

- **Is `Unicode-3.0` acceptable indefinitely?** It is permissive and appears via
  27 crates in the ICU tree pulled in transitively. Allowed here; if the
  licensing posture is ever reviewed by counsel, this is the entry to raise.
