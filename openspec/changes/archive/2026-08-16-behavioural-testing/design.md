## Context

See `proposal.md` — Why. Four facts about the current tree shape the approach,
all verified this session rather than assumed.

- The workspace has exactly one dev-dependency, `pdf-extract` in
  `adocpdf-cli`. There is no testing infrastructure beyond `cargo test`.
- The architecture guard reads `[dev-dependencies]` alongside `[dependencies]`,
  and `architecture.toml` says why: *"a test that reaches outward breaks the
  layering just as a library that does"*. Adding `proptest` to two crates is
  therefore an edit to `architecture.toml`, by design.
- `rust-toolchain.toml` pins stable `1.97.1`. `cargo-fuzz` needs nightly.
- `deny.toml`'s licence allow-list is exhaustive and deliberately excludes
  everything not surveyed. One of the crates this change would introduce is not
  on it, and not by oversight — see D6.

## Goals / Non-Goals

**Goals:**

- Turn three claims of the form *for all inputs* into tests of that form.
- Keep the everyday `cargo test` and `scripts/ci/gate.sh` fast enough that
  nobody is tempted to skip them.
- Make every finding land somewhere durable: a fuzzer crash that is fixed and
  forgotten is a bug waiting to return.

**Non-Goals:**

- Whole-workspace mutation coverage. See D8 — enforced narrowly, reported
  broadly.
- Fuzzing the real filesystem. See D7.
- Any change to production code. If a property test or fuzz target finds a real
  defect, that is a finding to report, and fixing it is a separate change.

## Decisions

### D1 — `proptest` 1.11.0, as a dev-dependency of two crates

Licence `MIT OR Apache-2.0`; the allow-list in `deny.toml` selects Apache-2.0,
which is already the project's own licence. No allow-list edit is needed for
`proptest` itself — but it arrives with a transitive tree (`rand`, `bit-set`,
`regex-syntax`, `rusty-fork`, `unarray` among others) that has **not** been
surveyed. Confirming that tree against the allow-list is a task, run before the
dependency is considered accepted, not a footnote afterwards.

Chosen over `quickcheck`, which is less actively maintained and whose shrinking
is weaker — and shrinking is most of the value here, since the interesting
inputs to `string_literal` are strings where one character out of many matters.
Chosen over hand-rolled `arbitrary` loops because that is a test framework
written in-house, which `AGENTS.md` rules out.

`architecture.toml` gains `proptest` in the `external` list of both
`adocpdf-domain` and `adocpdf-infra`.

### D2 — The injection property is tested in two tiers

The claim is that for any input, `string_literal` produces a literal the engine
displays verbatim. Testing it needs an oracle for "displays verbatim", and the
two available oracles have opposite strengths.

**Tier 1 — cheap, thousands of cases.** Structural invariants plus a round trip
through a small inverse decoder written in the test: the output opens and closes
with an unescaped quote, contains no unescaped quote or backslash between them,
contains no raw control character, and decodes back to exactly the input. Fast
enough to run on every `cargo test`.

The decoder is a test oracle, not a second implementation. `AGENTS.md`'s "Do not
write a checker" rule is about replacing an existing analyzer with bespoke
production tooling; an inverse function that exists to disagree with the forward
one is how round-trip properties are written, and no off-the-shelf tool knows
this codebase's output format.

**Tier 2 — expensive, few cases.** Tier 1 proves the output matches *our* model
of an engine string literal. Only the engine can prove it matches the engine's.
So a second property, configured to a low case count, emits the literal, runs it
through the real compilation path already used by the renderer, and asserts the
extracted text equals the input.

Neither tier alone is sufficient: Tier 1 would pass a systematic
misunderstanding of Typst's grammar, and Tier 2 alone runs too few cases to
explore. Running both is the point.

### D3 — The sandbox property is tested against the rule, not the filesystem

`adocpdf-domain::sandbox` is filesystem-free by construction: it takes resolved
locations through the `PathResolver` port. The property follows that seam. A
generator builds path expressions from segments — ordinary names, `.`, `..`,
absolute prefixes, empty components, and the surprising ones like a trailing
`..` that cancels the segment before it — and drives them through a hand-written
in-memory fake resolver, per the project's convention of fakes over mocks.

The invariant is stated as the codebase states it: acceptance is a function of
where the path resolves and of nothing else. Two expressions resolving to the
same location are accepted or refused alike, however differently they are
spelled.

What this deliberately does **not** cover is symlink resolution, which is the
adapter's behaviour and is real I/O. Those stay as integration tests in
`adocpdf-infra`, where they already are. Pretending a domain property test
covers a filesystem behaviour would be the more dangerous outcome than not
having one.

### D4 — Property tests use random seeds, and that does not violate determinism

`AGENTS.md` requires that identical inputs produce byte-identical output, and a
careless reading forbids a randomised test generator. The requirement governs
`adocpdf`'s output for a given document, not the test harness's choice of which
documents to try. Fixing the seed would convert a finder into a fixed test suite
wearing a generator's clothes, which is the whole value gone.

So: seeds vary, and `proptest`'s regression files — the `proptest-regressions/`
directory, where it persists every failing input it has ever found — are
**committed**. A failure found once is replayed on every run thereafter, on
every machine.

The consequence is stated plainly rather than discovered: a property test can
fail on a run where the previous run passed. That is a real defect found by a
new input, not flake, and it must never be "fixed" by re-running CI.

### D5 — Fuzzing lives outside the stable toolchain and outside the gate

**Verified:** `cargo-fuzz` requires a nightly compiler because it depends on
LLVM sanitizer instrumentation, which is unstable and, per the upstream
documentation, not close to stabilising. Sanitizer support is also
Linux-oriented.

Three consequences, taken deliberately:

1. **`fuzz/` is kept out of the root workspace — deliberately, by us.**

   **Corrected during apply.** This point previously read "`fuzz/` is its own
   workspace, as `cargo fuzz init` generates it". That is false, and has been
   since cargo-fuzz 0.11.4 (January 2024): the generated `fuzz/Cargo.toml` no
   longer carries a `[workspace]` table, and the parent manifest is not
   touched. Isolation is therefore something this change must *create*, not
   something it inherits — which matters, because D6's entire licence argument
   rests on it.

   Two further consequences of this repository's shape, also found during
   apply. The root manifest is **virtual** (`[workspace]` with no `[package]`),
   so the `path = ".."` dependency `cargo fuzz init` generates cannot resolve
   and must be pointed at a real crate by hand. And `members` is
   `["crates/*", "xtask"]`, which does not match `fuzz/`, but an unexcluded
   nested manifest is still an error the moment cargo notices it — so the root
   gains an explicit `exclude = ["fuzz"]` rather than relying on the glob
   happening not to match.
2. **Fuzzing is not a gate job.** `scripts/ci/gate.sh` runs on the pinned stable
   toolchain and every job in it terminates. Fuzzing does neither. It runs as a
   scheduled CI workflow on nightly with a per-target time budget.
3. **The value is carried back to stable through regressions.** Every crash the
   fuzzer finds is minimised and committed as an ordinary `#[test]` in the
   normal suite. The fuzzer is the finder; the stable suite is the record. A
   contributor who never installs nightly still runs every bug fuzzing has ever
   found.

Rejected: `afl.rs`, which would trade one nightly dependency for a different
toolchain story without removing the sanitizer problem; and skipping fuzzing
because of the nightly requirement, which would leave the untrusted-input claim
resting on examples.

Note that `taiki-e/install-action` does **not** carry `cargo-fuzz` — verified
against its `TOOLS.md`, where `cargo-mutants` does appear. The scheduled
workflow installs it with `cargo install cargo-fuzz --locked`; it is the one
tool in the project not available as a prebuilt binary, and the slower install
is acceptable in a job that already runs for minutes.

### D6 — `libfuzzer-sys` needs a licence the allow-list does not grant, and the allow-list does not change

**Verified:** `libfuzzer-sys` 0.4.13 is licensed `(MIT OR Apache-2.0) AND
NCSA`. The `AND` matters. Every other multi-licence crate in this tree is a
disjunction, where `deny.toml`'s allow-list picks the permissive branch —
`self_cell` and `r-efi` are already documented that way. Here NCSA is
conjunctive: it cannot be declined by choosing the other side.

NCSA is absent from `deny.toml`. Two ways out:

- **Add NCSA to the allow-list.** Rejected. NCSA is permissive — it is the LLVM
  licence — but adding it grants it to *everything*, including a future
  dependency that ends up inside the shipped binary. The allow-list's stated
  purpose is that admitting a licence is a deliberate, reviewed edit; widening
  it to accommodate a test tool that never ships would spend that guarantee on
  the least important consumer of it.
- **Keep `fuzz/` out of the root workspace**, so `cargo deny` never evaluates
  it. D5 arranges this — though note the correction recorded there: the
  isolation is not inherited from `cargo fuzz init`, it is created by an
  explicit `exclude = ["fuzz"]` in the root manifest. The licence argument
  below depends on that line actually being present, so it is a task step with
  a verification, not an assumption.

The second is chosen — but *relying on invisibility is not a policy*. So
`deny.toml` gains a comment recording that the fuzz crate is out of scope, that
this is intentional, that `libfuzzer-sys` would fail the allow-list if it were
in scope, and on what condition the decision should be revisited. A future
reader who discovers an unaudited crate in the tree finds the reasoning instead
of an apparent oversight.

The same reasoning applies to the architecture guard, which globs `crates/*` and
therefore never sees `fuzz/` either. `architecture.toml` gets the equivalent
note. **Unverified:** whether the guard's `UngovernedCrate` check might still
reach a root-level `fuzz/` directory under some path — cheap to confirm by
running the guard once after `cargo fuzz init`, and a task says to.

### D7 — Fuzz targets exercise the pure path, never real I/O

The first target takes arbitrary bytes, interprets them as an AsciiDoc source,
and drives parse → plan → emit markup. No file is read, no file is written, no
PDF is laid out.

- **Correctness of the experiment.** A fuzzer that generates *paths* and hands
  them to real filesystem calls is not testing the sandbox; it is a program that
  writes attacker-controlled paths on the machine running it. The sandbox rule
  is pure and is tested purely, per D3.
- **Throughput.** Executions per second is what makes fuzzing work. Laying out a
  PDF per input would cut that by orders of magnitude.
- **It covers the boundary that matters.** The assertion is that for any input
  bytes, emission terminates without panicking and the markup it produces is
  well-formed — the injection claim, restated over adversarial input.

A second target over full in-memory rendering is a reasonable later addition and
is out of scope here.

### D8 — `cargo-mutants` is enforced narrowly and reported broadly

`cargo-mutants` 27.1.0, MIT. It rebuilds and reruns the suite once per injected
mutation; on a tree that embeds Typst, a whole-workspace run is far too slow for
`scripts/ci/gate.sh`, which must stay something a contributor runs before every
change.

It runs as a scheduled CI job, and its pass condition is deliberately asymmetric:

- **Fails** if any mutant survives in `crates/adocpdf-infra/src/markup.rs` or
  `crates/adocpdf-domain/src/sandbox.rs`. These are the two modules whose
  correctness the project's security claims rest on, they are small, and after
  D2 and D3 they should have no surviving mutants. A failure means a property
  test is not asserting what it appears to.
- **Reports only** for the rest of the workspace, as a published artifact.

The alternative — demand zero survivors workspace-wide — sounds stricter and is
worse. It is not achievable on a first run, so it would be disabled or
grandfathered with an exclusion list nobody revisits, and the enforcement would
be theatre. The alternative in the other direction, a scheduled job with no
failure condition at all, produces a report nobody reads, which is the same
outcome as not running it.

Exclusions live in `.cargo/mutants.toml`, each with a written reason, per
`AGENTS.md`. **Corrected during apply:** this said `mutants.toml` at the repo
root; cargo-mutants reads `.cargo/mutants.toml`. That directory already exists
here — it holds `audit.toml` — so the correction is a path, not a new
convention. Note also that `#[mutants::skip]` takes **no** `reason` argument;
a reason is a neighbouring comment, and the attribute would require the
`mutants` crate as a *regular* dependency. Config-file exclusion is preferred
precisely because it needs no dependency at all.

### D9 — Nothing here relaxes the coverage floor

`AGENTS.md` forbids lowering it, and mutation testing is not an argument for
doing so: the two measure different things, and a module can have no surviving
mutants because it has no tests reaching it at all. The floor stays at 90%
line coverage, workspace-wide. Mutation testing answers whether the covered
lines are actually asserted on.

## Risks / Trade-offs

- **`proptest`'s transitive tree is unsurveyed.** → A task runs `cargo deny
  check licenses` and `cargo audit` immediately after adding it, before the
  dependency is accepted. If a transitive crate needs a licence outside the
  allow-list, that is a decision for the user, not a quiet allow-list edit.

- **Slower test runs, and slower coverage runs in particular.** `cargo llvm-cov`
  instruments everything, and thousands of generated cases per property are not
  free. → Tier 1 case counts are tuned to keep the gate usable; Tier 2 is
  explicitly low-count. If the gate becomes slow enough to be skipped, the
  property tests have made things worse, and the case counts come down.

- **The fuzzer will find bugs in `asciidoc-parser`, not only here.** An upstream
  panic on malformed input is indistinguishable from ours at the crash site. →
  Findings are triaged before being called defects; upstream ones are reported
  upstream, and this project's spec already requires that malformed source not
  be fatal, so an upstream panic reaching the user is our problem to contain
  even when it is not our bug.

- **Nightly breaks.** `libfuzzer-sys` against a moving nightly goes red for
  reasons unrelated to this codebase. → The fuzz workflow is scheduled and
  separate; it never blocks a merge. A red fuzz job is investigated, not
  ignored, but it does not stop work.

- **Mutation testing may show the 96% coverage is shallow.** → That is the
  finding, not a failure of the change. Only `markup.rs` and `sandbox.rs` are
  enforced; a poor result elsewhere is information for a later change.

- **`fuzz/` sits outside both the architecture guard and the licence policy.** →
  Accepted per D5 and D6, and documented in both `architecture.toml` and
  `deny.toml` so it reads as a decision rather than a gap. It is never linked
  into a shipped artefact and never runs in the gate.

## Open Questions

- The per-target fuzz time budget in the scheduled workflow. Five minutes is a
  reasonable start; the right number comes from watching how quickly coverage of
  the corpus plateaus, and changing it touches one YAML line.
- Whether Tier 2's engine round trip belongs in the normal suite or only in the
  scheduled run, if it proves slower than expected. Measurable at apply time,
  and it changes no decision above.
