## 1. The gate job

- [x] 1.1 Add a `commit_lint` shell function to `scripts/ci/gate.sh` that
      resolves the base revision — `refs/remotes/origin/HEAD`, then
      `origin/main`, then `main`, first that exists — and runs
      `committed "<base>..HEAD" --no-merge-commit`. When none of the three
      resolves, print what was tried and return non-zero. Comment it with
      design D2's reasoning, in the style of the file's existing functions.
      Verified by `bash -n scripts/ci/gate.sh` and by calling the function
      directly on this branch: it must resolve a base and lint the two
      commits this branch adds.

- [x] 1.2 Register the job as
      `run_tool_job "commit convention" committed "cargo install committed --locked" commit_lint`
      after the existing tool-backed jobs. Verified by running the gate and
      seeing an eighteenth job named `commit convention` reach `pass`, and by
      temporarily renaming `committed` on `PATH` and seeing the job FAIL with
      its install command rather than skip.

- [x] 1.3 Prove the job actually catches the defect that motivated this
      change: on a scratch branch, commit with a body line wrapped at 77
      characters, run the gate, and confirm `commit convention` FAILS with
      `Line is too long`. Then confirm a run with an empty commit range
      (on `main`) passes. Delete the scratch branch. Record both outcomes in
      the task list when ticking it off — this is the only verification that
      the job is wired to a real rule and not to a vacuous one.

## 2. CI

- [x] 2.1 Give `.github/workflows/ci.yml`'s checkout `fetch-depth: 0`, with a
      comment saying the gate now reads history and why the whole of it is
      needed. Leave `persist-credentials: false` and its comment in place.
      Verified by `actionlint` and `zizmor` via the gate.

- [x] 2.2 Add an `Install committed` step to `ci.yml` beside the existing
      `Install actionlint` step, running
      `cargo install committed --version 1.1.11 --locked`, commented as the
      other tool `taiki-e/install-action` does not carry. Verified by
      `actionlint` and `zizmor` via the gate, and by the gate workflow
      running green on the pull request that lands this change.

- [x] 2.3 Delete `.github/workflows/commits.yml`. Do this only after 2.1 and
      2.2 are in the same commit or an earlier one, per the design's ordering
      constraint. Verified by the pull request showing one workflow where it
      previously showed two, with `quality gate` still required and now
      covering commit messages.

## 3. Documentation

- [x] 3.1 Rewrite `AGENTS.md`'s section "The one check that is not in the
      gate". It becomes the record of a reversed decision: what was decided,
      why it was reversed, what an empty commit range means, that a gate run
      before committing still cannot see a message written after it, and that
      the local-hook rejection stands untouched. Update the job count from
      seventeen to eighteen there and add `committed` to its install list.
      Verified by reading the section against `proposal.md`'s "Reopening a
      recorded decision" — no sentence in it may still claim the check is
      outside the gate.

- [x] 3.2 Update `CONTRIBUTING.md`: the gate now covers commit messages, so
      the standalone `committed <base>..HEAD` instruction becomes the
      narrower "check one range by hand" tool rather than a step before
      opening a pull request. Add `committed` to the tool-install block at
      line 47, and state the 72-character wrap width without explaining the
      tolerance (design D5). Verified by following the document from a clean
      checkout: the tools it names must be exactly the tools the gate needs.

- [x] 3.3 Update `README.md`'s "seventeen jobs" and its tool list to include
      `committed`. Verified by `grep -rn "seventeen" README.md AGENTS.md`
      returning nothing outside `openspec/changes/archive/`.

## 4. Gate

- [x] 4.1 Run `scripts/ci/gate.sh` and confirm all eighteen jobs pass,
      including the new one. Do not reduce a check, weaken an assertion or
      lower the coverage floor to get there. Then confirm the gate's own
      commits for this change pass `commit convention`, which is the change
      verifying itself.
