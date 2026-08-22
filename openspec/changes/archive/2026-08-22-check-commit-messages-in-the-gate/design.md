## Context

See `proposal.md` — Why. The constraints that shape the approach:

- `scripts/ci/gate.sh` already has `run_tool_job`, which fails a job with its
  install command when the tool is absent. The gate's header states the rule
  it enforces: "A job whose tool is missing FAILS. It never skips."
- `.github/workflows/ci.yml` runs `scripts/ci/gate.sh` as a single step,
  under the comment "One command. Every check, in the order and with the
  messages a contributor sees locally." A check that exists only in a
  separate workflow contradicts that sentence.
- `main`'s branch protection lists one required status check, `quality gate`
  (verified against the GitHub API on 2026-08-22). `conventional commits` is
  absent from it.
- `ci.yml` checks out at the default depth of 1. There is no history for a
  commit-message linter to read.
- `committed` 1.1.11 is installed by `cargo install committed --locked`;
  `taiki-e/install-action` v2.86.1 does not carry it, per its TOOLS.md.

`committed` is a development tool invoked as a binary, not a dependency of
any crate. It enters no `Cargo.toml`, so `architecture.toml` and the
architecture guard in `xtask/tests/architecture.rs` do not see it and must
not: the guard checks the inward dependency rule between crates, and no crate
gains an edge here. No third-party crate is added to the workspace.

**No injection, sandbox or determinism boundary is touched.** Nothing in this
change reaches `parser::refuse_input_that_could_forge_structure`,
`markup::string_literal`, `SafeMode::Secure`, or `ReferenceTime`. The two
architectural invariants are untouched because no rendering code is.

## Goals / Non-Goals

**Goals:**

- A contributor who runs `scripts/ci/gate.sh` after committing learns about a
  malformed commit message before pushing.
- One definition of the rule, so the local run and CI cannot disagree about
  what passes.
- The check becomes blocking without editing branch protection.

**Non-Goals:**

- Guaranteeing the gate catches every message. A gate run before a commit is
  written cannot see the message written after it; see D3.
- Reproducing GitHub's pull-request merge ref locally. The gate resolves a
  base branch; see D2 for why that is equivalent and where it is not.

## Decisions

### D1: A `run_tool_job` in `gate.sh`, not a new script

`run_tool_job "commit convention" committed "cargo install committed --locked"`
places the job on the same footing as the other nine tool-backed jobs: same
failure format, same missing-tool message, same accumulation into `failed`.

*Alternative — a separate `scripts/ci/commits.sh` the gate calls.* Rejected:
`gate.sh` already inlines every other job's invocation, and a one-command job
does not earn a file. The base-revision lookup does need a shell function,
which is how `wasm_build`, `docs_build` and `msrv_build` are already handled.

### D2: The range is `<base>..HEAD`, base resolved by falling back

CI's deleted workflow used `HEAD~..HEAD^2`, which only exists on GitHub's
synthesised merge commit. Locally the equivalent set is every commit the
branch adds over the branch it will merge into.

Resolution order, first that exists wins:

1. `git symbolic-ref --quiet refs/remotes/origin/HEAD` — the remote's own
   statement of its default branch, so a repository that renames `main` keeps
   working without editing the gate.
2. `origin/main`.
3. `main`.

If none resolves, the job **fails** with an explanation. It does not skip.
That is the gate's stated rule and the reason it is stated: a skip here would
report success on a checkout that verified no commit.

The lint is then `committed "<base>..<tip>" --no-merge-commit`, where `<tip>`
is `HEAD^2` when HEAD is a merge commit and `HEAD` otherwise. See "The merge
commit" below for why that distinction is not optional.

*Alternative — hard-code `origin/main..HEAD`.* Rejected: a contributor
working from a fork has `origin` pointing at their own copy, and a renamed
default branch breaks it silently.

*Alternative — `git merge-base` and lint from there.* `<base>..HEAD` already
means "reachable from HEAD, not from base", which is the merge-base range.
The extra call would buy nothing.

**Where this is not equivalent:** if a contributor's `origin/main` is stale,
the range includes commits that are already on the real `main` and have
already passed. They pass again. The failure mode is redundant work, not a
false verdict.

**The merge commit.** This was first written as `<base>..HEAD` on the
reasoning that `--no-merge-commit` would drop GitHub's synthesised merge
commit, the way `merge_commit`'s name in `committed.toml` suggests. That
reasoning was asserted, not run, and the first CI run of the job refuted it:

```
69d4f4c: error Commit is not in Conventional format: Missing type …
69d4f4c: error Merge commits are disallowed
```

Measured against `committed` 1.1.11: `merge_commit = false` **refuses** a
merge commit, and `merge_commit = true` allows one to exist while still
holding its generated subject to the convention. Neither value skips one. The
deleted workflow never met this because `HEAD~..HEAD^2` excludes the merge
structurally rather than by any option.

So the job reaches past it the same way: when HEAD has a second parent, the
branch tip is `HEAD^2`, and `<base>..HEAD^2` is exactly the commits the pull
request adds. Locally HEAD is not a merge and nothing changes. Verified by
synthesising the merge ref locally — `git merge --no-ff` from `origin/main` —
where the fixed job passes and the original expression reproduces both errors
above. On a push to `main` the range is empty.

`--no-merge-commit` is kept, and now it means something: `main` requires
linear history, so a merge commit *inside* a branch is a mistake and the job
names it.

### D3: An empty commit range passes, and that is a check, not a skip

Running the gate on `main`, on a detached HEAD at a merged commit, or before
making any commit gives an empty range. `committed` exits 0 — verified
against 1.1.11 by running it on `origin/main..origin/main`.

This is the point the reversed decision in `AGENTS.md` turned on, so it is
worth being exact. The gate's objection is to a check that "quietly skips",
because that "reports success on a machine that verified nothing". A job that
examines a range of zero commits has examined every commit in front of it.
Nothing is unverified; there is nothing there.

What it does *not* do is warn you about a message you have not written yet.
The gate is normally run again after committing and before pushing, which is
where the value is, and the documentation must say this plainly rather than
imply the gate now guarantees a green `commits` check.

### D4: `commits.yml` is deleted rather than kept alongside

Keeping it would mean two expressions of one rule, two `cargo install
committed` compilations per pull request, and a standing risk that the ranges
drift apart — the exact failure this change exists to remove. Deleting it
also makes the rule blocking, because `quality gate` is the required check
and `conventional commits` never was.

*Alternative — add `conventional commits` to branch protection and keep both.*
Rejected: it fixes the enforcement gap without fixing the locality gap, and
leaves the duplication in place.

**What is lost:** a red `conventional commits` check named its failure in the
GitHub UI without opening logs. After this, a message defect fails the gate
job named `commit convention`, one line in the gate's output. The trade is a
less specific check name for a check that actually blocks.

### D5: `committed.toml` is not touched

The 72-character body limit is `committed`'s default. AGENTS.md forbids
silencing a lint, and `committed.toml`'s own header states that every value
departing from the defaults is recorded with a reason — there is no reason to
record here beyond "the messages were long", which is not one.

Worth writing down, because it is not obvious and cost time to find:
`line_length` is measured on the line **excluding its final word**, so with
`line_length = 72` a prose line of 75 characters passes and 76 fails, and a
line ending in a long unwrappable token (a URL) is exempt entirely. Measured
against `committed` 1.1.11 with this repository's `committed.toml`:

| line | last word | verdict |
|---|---|---|
| 75 chars, prose | short | passes |
| 76 chars, prose | 2 chars | fails, "76 exceeds the max length of 72" |
| 76 chars, prose | 10 chars | passes |
| 86 chars, `short ` + 80-char token | 80 chars | passes |

Wrapping at 72 always passes. The documentation should say 72 and not try to
explain the tolerance.

### D6: `ci.yml` gets `fetch-depth: 0`, and keeps `persist-credentials: false`

Full history is what the job needs; the deleted workflow already used
`fetch-depth: 0` for the same reason. `persist-credentials: false` stays and
its comment stays true — the gate reads the tree and now also reads history,
and writes neither.

The `committed` install goes in its own step next to the `actionlint` one,
which is already the established place for "the tool `install-action` does
not carry". `--locked` pins its dependency tree; the version is pinned to
1.1.11, the version the deleted workflow pinned, so nothing about what CI
enforces changes on the day upstream releases 1.2.

## Risks / Trade-offs

- **The gate gets slower and needs one more tool** → `committed` compiles in
  around 90 seconds on a cache miss and is cached by `rust-cache` after that.
  Net CI time falls, because the workflow that paid this cost is deleted.
- **A contributor without `committed` can no longer run the gate at all** →
  it is already listed as required before opening a pull request, so the tool
  set does not grow. The gate fails with the exact install command, which is
  what `run_tool_job` exists for.
- **A stale `origin/main` re-lints landed commits** → they pass; the cost is
  seconds. Stated in D2 rather than engineered around.
- **A fork with no `origin/HEAD`, no `origin/main` and no `main`** → the job
  fails with an explanation rather than passing vacuously. Loud is correct
  here: it is a genuinely unusual checkout and the alternative is a silent
  pass.
- **`commit convention` disappears from the PR checks list** → anyone
  watching for that name sees it vanish. Called out in D4; the failure still
  appears, inside `quality gate`.
- **The gate can still be run before committing and pass** → D3. Narrowed,
  not closed, and documented as narrowed.

## Migration Plan

No data, no schema, no released interface. The steps are: add the job, prove
it fails on a bad message and passes on a good one, then delete the workflow
and update the four documents that describe the old arrangement. Rollback is
`git revert` — restoring `commits.yml` restores the previous behaviour
exactly, and branch protection never changed, so there is no settings state
to unwind.

The one ordering constraint: `commits.yml` must not be deleted before
`ci.yml` fetches full history and installs `committed`, or a pull request
opened in between is checked by nothing.
