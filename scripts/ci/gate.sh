#!/usr/bin/env bash
#
# The pre-merge quality gate.
#
# Every job here must pass before a change is considered done. A green
# typecheck is not a passing test suite: the jobs below are the standard, and
# skipping one is not a judgement call.
#
# A job whose tool is missing FAILS. It never skips. A check that quietly skips
# reports success on a machine that verified nothing, which is worse than having
# no check at all, because it is indistinguishable from a real pass.
#
# Run from anywhere: scripts/ci/gate.sh

set -euo pipefail

# Declared and assigned separately on purpose: `readonly VAR="$(cmd)"` returns
# the status of `readonly`, not of the command, so a failing `cd` here would be
# swallowed despite `set -e`. Found by shellcheck (SC2155).
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly ROOT
cd "$ROOT"

# Crates that must keep compiling for the browser target. Delivery and
# infrastructure are excluded for now: whether the layout engine's dependency
# tree builds for WASM is an open question, deliberately unanswered in the
# render-first-pdf change.
readonly WASM_CLEAN_CRATES=(
    adocpdf-core
    adocpdf-domain
    adocpdf-shared
    adocpdf-wasm
)

# The minimum line coverage the workspace must reach.
readonly MIN_LINE_COVERAGE=90

# The oldest toolchain the workspace claims to support. Declared in every
# manifest as `rust-version`; checked here so the claim stays true.
readonly MSRV=1.92

failed=()

run_job() {
    local name="$1"
    shift
    printf '\n\033[1m==> %s\033[0m\n' "$name"
    if "$@"; then
        printf '\033[32m    pass\033[0m\n'
    else
        printf '\033[31m    FAIL\033[0m\n'
        failed+=("$name")
    fi
}

# Runs a job that depends on an installed tool, failing with the install
# command when the tool is absent.
run_tool_job() {
    local name="$1" tool="$2" install="$3"
    shift 3
    printf '\n\033[1m==> %s\033[0m\n' "$name"

    if ! command -v "$tool" >/dev/null 2>&1; then
        printf '\033[31m    FAIL — %s is not installed\033[0m\n' "$tool"
        printf '    install it with:  %s\n' "$install"
        failed+=("$name (tool missing)")
        return
    fi

    if "$@"; then
        printf '\033[32m    pass\033[0m\n'
    else
        printf '\033[31m    FAIL\033[0m\n'
        failed+=("$name")
    fi
}

wasm_build() {
    local crate
    for crate in "${WASM_CLEAN_CRATES[@]}"; do
        cargo build --quiet -p "$crate" --target wasm32-unknown-unknown || return 1
    done
}

docs_build() {
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --quiet
}

coverage() {
    cargo llvm-cov --workspace --summary-only \
        --fail-under-lines "$MIN_LINE_COVERAGE"
}

msrv_build() {
    if ! rustup toolchain list | grep -q "^${MSRV}"; then
        printf '    the %s toolchain is not installed\n' "$MSRV"
        printf '    install it with:  rustup toolchain install %s --profile minimal\n' "$MSRV"
        return 1
    fi
    cargo "+${MSRV}" check --workspace --all-targets --quiet
}

# --- correctness -------------------------------------------------------------

run_job "formatting"      cargo fmt --all -- --check
run_job "lints"           cargo clippy --workspace --all-targets -- -D warnings
run_job "tests"           cargo test --workspace
run_job "architecture"    cargo test -p xtask --test architecture
run_job "wasm build"      wasm_build
run_job "docs"            docs_build
run_job "msrv (${MSRV})"     msrv_build

# --- static analysis ---------------------------------------------------------

run_tool_job "shell" shellcheck \
    "sudo apt install shellcheck" \
    shellcheck "$0"

run_tool_job "toml" taplo \
    "cargo install taplo-cli --locked" \
    taplo fmt --check

run_tool_job "spelling" typos \
    "cargo install typos-cli --locked" \
    typos

run_tool_job "feature combinations" cargo-hack \
    "cargo install cargo-hack --locked" \
    cargo hack check --workspace --feature-powerset --no-dev-deps --quiet

# The workflow files run on every push and carry a token. Everything else in
# this repository is linted; leaving the one file with those properties
# unchecked would be the wrong place to make an exception.
#
# Two tools, because they answer different questions. `actionlint` is about
# correctness — expression syntax, invalid runner labels, and shellcheck over
# the `run:` blocks, which are otherwise the only shell here this gate does not
# see. `zizmor` is about security — script injection through untrusted
# interpolation, over-broad permissions, unpinned actions.
run_tool_job "workflow syntax" actionlint \
    "download from https://github.com/rhysd/actionlint/releases" \
    actionlint

run_tool_job "workflow security" zizmor \
    "cargo install zizmor --locked" \
    zizmor --quiet .github/workflows

# --- dependency hygiene ------------------------------------------------------

run_tool_job "advisories" cargo-audit \
    "cargo install cargo-audit --locked" \
    cargo audit

run_tool_job "licences" cargo-deny \
    "cargo install cargo-deny --locked" \
    cargo deny check licenses bans sources

run_tool_job "unused deps" cargo-machete \
    "cargo install cargo-machete --locked" \
    cargo machete

# --- coverage ----------------------------------------------------------------

run_tool_job "coverage (>=${MIN_LINE_COVERAGE}% lines)" cargo-llvm-cov \
    "cargo install cargo-llvm-cov --locked" \
    coverage

# -----------------------------------------------------------------------------

printf '\n'
if [[ ${#failed[@]} -gt 0 ]]; then
    printf '\033[31mgate failed: %s\033[0m\n' "${failed[*]}"
    exit 1
fi

printf '\033[32mgate passed\033[0m\n'
