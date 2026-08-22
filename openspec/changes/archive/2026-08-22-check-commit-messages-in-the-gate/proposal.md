## Why

The gate is meant to be the one command that answers "will CI accept this".
On pull request #4 it answered wrong: `scripts/ci/gate.sh` reported all
seventeen jobs green — 505 tests, 95.52% line coverage — while
`.github/workflows/commits.yml` failed, because `committed` caps a commit
body line at 72 characters and one message was wrapped at 77. Nothing a
contributor could run locally would have said so.

The command that would have caught it exists and is documented twice
(`AGENTS.md:206`, `CONTRIBUTING.md:105`). That is the problem: it is a
*second* command, remembered separately from the gate, and a step that has to
be remembered separately is a step that gets skipped. The evidence is this
repository's own history — the instruction was written, read, and not run.

There is a second, worse fact behind it. `main`'s branch protection requires
exactly one status check, `quality gate`; `conventional commits` is not on
the list. Pull request #4 was therefore mergeable with that check red. The
commit-message rule this project mandates in `AGENTS.md` and
`openspec/config.yaml` has never actually blocked anything.

## What Changes

- Add an eighteenth job, `commit convention`, to `scripts/ci/gate.sh`. It
  uses the existing `run_tool_job` helper, so a missing `committed` FAILS the
  gate with its install command rather than skipping, exactly as the other
  tool-backed jobs do.
- The job lints the commits the current branch adds over the default branch —
  the same set `commits.yml` lints on a pull request, expressed against a
  local branch instead of GitHub's synthesised merge commit.
- Rewrite `AGENTS.md`'s section "The one check that is not in the gate". Its
  claim stops being true, and its reasoning is answered rather than deleted:
  what an empty commit range means, and what this still does not close.
- Delete `.github/workflows/commits.yml`. Once `gate.sh` runs the check, that
  workflow is a second definition of the same rule and the only one that is
  not required to pass — so the check becomes blocking for the first time,
  with no branch-protection change.
- Give `.github/workflows/ci.yml` the history and the tool the new job needs:
  `fetch-depth: 0` on checkout, and `cargo install committed --locked`
  alongside the existing `actionlint` step, `committed` being the other tool
  `taiki-e/install-action` does not carry.
- Update `CONTRIBUTING.md`'s standalone `committed` instruction to point at
  the gate, keeping the direct invocation as the narrower tool it now is.
- Update the job count in `README.md` and `AGENTS.md`, both of which state
  "seventeen".

## Reopening a recorded decision

`AGENTS.md:190-201` records a deliberate decision that this check is *not* in
the gate, with two reasons. This change reopens the first and leaves the
second standing.

**Reopened.** "The gate checks the working tree; a commit-message linter
checks history, which the gate has no view of. Putting it there would add a
job that is meaningless in the situation the gate is normally run in." The
premise is right and the conclusion does not follow. A gate run before any
commit exists gives the job an empty commit range, and an empty range is a
complete check of zero commits — not a skip. The gate's own header draws
exactly that line: it objects to a check that "quietly skips" because such a
check "reports success on a machine that verified nothing". A job that
examines every commit in a range of zero has verified everything in front of
it. The distinction the gate cares about is met.

What the decision correctly foresaw is that the job is *uninformative* before
a commit exists. That is a real cost and it is smaller than the cost paid on
pull request #4, because the gate is normally run again after committing and
before pushing.

**Untouched.** "A local hook was rejected — `--no-verify` exists, so a check
the author can skip is documentation." That still holds and this change does
not install a hook. The gate is not a hook: it is not wired into `git commit`
and has no flag that bypasses it. Enforcement stays in CI — it simply moves
from a workflow nothing requires into the one workflow that is required.

## Capabilities

### New Capabilities

None. No crate behaviour changes.

### Modified Capabilities

None. This touches repository tooling and contributor documentation only, so
`.openspec.yaml` sets `skip_specs: true`.

## Non-goals

- **Enforcing commit messages locally.** The local run is an early warning.
  What blocks a merge is still CI, on a machine the project controls; only
  which workflow carries it changes.
- **Changing branch protection.** `quality gate` is already the one required
  check and stays that way. This change is deliberately shaped to need no
  settings edit.
- **Installing a git hook.** Rejected before, still rejected, same reason.
- **Changing `committed.toml`.** The 72-character body limit is `committed`'s
  default and no departure from it is being proposed. AGENTS.md forbids
  silencing a lint, and rewrapping prose is the honest response to this one.
- **Closing the gap completely.** A gate run *before* the commit is written
  cannot see the message written after it. This change narrows the window; it
  does not eliminate it, and the documentation must say so plainly rather
  than claim a guarantee the tooling does not provide.
- **Widening `WASM_CLEAN_CRATES`.** A separate, now-justified change.

## Impact

**Architectural layers: none.** No crate is touched — not `core`, `domain`,
`shared`, `infra`, `cli` or `wasm`. The inward dependency rule is unaffected,
and so are the two architectural invariants.

**Files:**

- `scripts/ci/gate.sh` — one new `run_tool_job` invocation and its helper for
  resolving the base revision.
- `AGENTS.md` — the "one check that is not in the gate" section, the gate job
  count, and the tool-install list.
- `CONTRIBUTING.md` — the `committed` instructions and tool-install list.
- `README.md` — the gate job count and its tool list.
- `.github/workflows/ci.yml` — checkout depth and one install step.
- `.github/workflows/commits.yml` — deleted. Its reasoning, which asserts the
  check does not belong in the gate, moves to `AGENTS.md` as the record of a
  decision that was reversed.

**Contributors:** `committed` becomes required to run the gate. It is already
documented as required before opening a pull request, so the set of tools a
contributor needs does not grow; the point at which a missing one is noticed
moves earlier.

**CI:** one workflow fewer and one job more. The gate workflow pays a
`cargo install committed --locked` on a cache miss — the same cost
`commits.yml` paid, now paid once instead of alongside it — and a full-history
checkout. Net, a pull request runs one workflow rather than two.

**Enforcement:** strictly stronger. A commit-message defect currently cannot
block a merge; after this it cannot reach one.
