## 1. Establish a baseline before committing anything

- [x] 1.1 Run `scripts/ci/gate.sh` against the current working tree and report
      the result job by job. *Verified by:* the gate runs to completion and its
      outcome is reported. If any job fails, report it and stop — committing a
      tree that does not pass its own gate would put the first commit in the
      history permanently below the standard every later commit must meet.
- [x] 1.2 Confirm `git status --porcelain` shows no build or coverage artefacts
      after that run, so the expanded `.gitignore` is doing its job before
      anything is staged. *Verified by:* the only untracked entries are real
      source files; `target/`, `*.profraw` and `lcov.info` are absent from the
      list.

## 2. Version control

- [x] 2.1 Commit the `render-first-pdf` work — the six crates under `crates/`,
      `xtask/`, `architecture.toml`, `Cargo.toml`, `Cargo.lock` and
      `rust-toolchain.toml` — as one Conventional Commit whose body states that
      this history is a reconstruction of an existing working tree rather than a
      replay of development (design D7). *Verified by:* `git log --stat` shows
      the commit; `git ls-files Cargo.lock` lists it, confirming the lockfile is
      tracked per design D6.
- [x] 2.2 Commit the `quality-gates` work — `scripts/ci/gate.sh`, `deny.toml`,
      `clippy.toml`, `rustfmt.toml`, `taplo.toml`, `_typos.toml`,
      `.cargo/audit.toml`, `.gitignore`, `LICENSE`, `LICENSING.md`, `README.md`,
      `AGENTS.md` — as a second Conventional Commit. *Verified by:* `git log
      --oneline` shows two commits and `git status --porcelain` no longer lists
      those paths.
- [x] 2.3 Commit the OpenSpec planning artifacts (`openspec/`) and the agent
      tooling directories (`.claude/`, `.agents/`). *Verified by:* `git
      ls-files | wc -l` is non-zero and covers the whole tree; `git status
      --porcelain` is empty apart from files this change has yet to create.

## 3. Contributor and security paperwork

- [x] 3.1 Add `SECURITY.md` stating the four boundaries from design D9 — the
      injection boundary through `markup::string_literal`, sandbox containment,
      panics or non-termination on untrusted input, and the upstream referral
      for Typst and `asciidoc-parser` findings — plus the reporting channel.
      *Verified by:* each of the four claims in the file corresponds to a
      boundary that exists in the code and is named in `AGENTS.md` or
      `README.md`; no claim is made that the codebase does not support.
- [x] 3.2 Add `CONTRIBUTING.md` covering the OpenSpec propose → apply → archive
      workflow, the requirement to run `scripts/ci/gate.sh` before a change is
      done, Conventional Commits, and the list of repository settings this
      change cannot apply (branch protection, required status checks, private
      vulnerability reporting). *Verified by:* every command it quotes runs as
      written from a clean checkout.
- [x] 3.3 Add `.editorconfig` matching the settings already implied by
      `rustfmt.toml` and `taplo.toml`, and a `CODEOWNERS` file. *Verified by:*
      `cargo fmt --check` and `taplo fmt --check` still pass, confirming
      `.editorconfig` contradicts neither formatter.

## 4. Workflow linting, added to the gate first

- [x] 4.1 Install `actionlint` and `zizmor`, and add both to
      `scripts/ci/gate.sh` as `run_tool_job` entries with their install commands
      in the failure message. *Verified by:* the gate reports both as passing
      with the tools present, and — checked by temporarily renaming a binary —
      reports FAIL with the install hint when one is absent, never a skip.
- [x] 4.2 Update the job count and the job table in `README.md` and `AGENTS.md`
      from fifteen to seventeen, adding a row for each new job (design D10).
      *Verified by:* the number stated in each file equals the number of
      `run_job`/`run_tool_job` calls in `scripts/ci/gate.sh`.

## 5. Continuous integration

- [x] 5.1 Add `.github/workflows/ci.yml` running `scripts/ci/gate.sh` on push
      and pull request against `ubuntu-latest`, with `permissions: contents:
      read`, `Swatinem/rust-cache`, and `taiki-e/install-action` supplying the
      gate tools. Every `uses:` is pinned to a full commit SHA with the version
      in a trailing comment (design D2, D4). *Verified by:* each pinned SHA is
      confirmed to resolve to the version named in its comment — checked, not
      copied from memory.
- [x] 5.2 Ensure the workflow installs what the gate needs beyond the tools: the
      `1.92` MSRV toolchain and the `wasm32-unknown-unknown` target. *Verified
      by:* every job in `scripts/ci/gate.sh` that depends on an external
      toolchain component has a corresponding provisioning step; cross-checked
      against `msrv_build` and `wasm_build` line by line.
- [x] 5.3 Install `actionlint` outside `install-action`, which does not carry it
      — verified against its `TOOLS.md` (design D2). *Verified by:* the CI
      workflow installs `actionlint` by a documented method and the `shell` and
      workflow-lint gate jobs pass on the runner's own configuration.
- [x] 5.4 Run `actionlint` and `zizmor` against the new workflow and fix every
      finding at its cause. *Verified by:* both tools exit clean. If any finding
      is genuinely wrong here, it is suppressed once with a written reason per
      `AGENTS.md`, never silently.

## 6. Dependency updates

- [x] 6.1 Add `.github/dependabot.yml` covering the `cargo` and
      `github-actions` ecosystems on a weekly schedule (design D5). *Verified
      by:* `actionlint` accepts the file and both ecosystems are present —
      `github-actions` is required, since D4's SHA pins have no other way to be
      updated.
- [x] 6.2 Record in `design.md` that Dependabot's handling of inherited
      `[workspace.dependencies]` entries is unverified, and note it as the first
      thing to check after the initial run. *Verified by:* the note exists and
      is marked unverified rather than asserted, per the project's design rules.

## 7. Close out honestly

- [x] 7.1 Run the full `scripts/ci/gate.sh` — now seventeen jobs — and confirm
      it passes. *Verified by:* the gate prints `gate passed`.
- [ ] 7.2 Report to the user that the workflow is **statically verified but has
      never executed**, and that its first real run requires a push, which
      `AGENTS.md` forbids without explicit consent (design D8). *Verified by:*
      the handover states plainly which parts are proven and which are not; no
      claim is made that CI is working.
