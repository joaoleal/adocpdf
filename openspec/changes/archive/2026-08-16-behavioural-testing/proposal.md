## Why

The project makes three claims that are quantified over *all* inputs, and tests
each of them with a handful of examples.

`README.md` leads with the strongest: *source content can never become a
rendering instruction*. The whole of that guarantee rests on one function,
`markup::string_literal` at `crates/adocpdf-infra/src/markup.rs:28`, whose own
doc comment argues that its escaping surface is "small, fixed, and provably
complete". Nothing proves it. `AGENTS.md` makes the second: *paths are judged by
where they resolve, not how they are spelled*. The third is in the spec already
— *malformed source still produces a document* — for input that is untrusted by
definition, since the point of the tool is to read files other people wrote.

Against that, the only measure of test quality in the repository is 96% line
coverage. Coverage says which lines ran. It cannot distinguish a test that
asserts the right answer from one that calls the function and throws the result
away, and it says nothing at all about the inputs nobody thought to write down.
Verified: the workspace has exactly one dev-dependency, `pdf-extract`, and no
property tests, fuzz targets or mutation runs anywhere.

The gap is specific, not general. Three claims of the form *for all inputs*,
tested by enumeration.

## What Changes

- **Property-based tests** (`proptest`) for the two for-all claims that have a
  single function or module behind them:
  - `markup::string_literal` — for any input string whatsoever, the emitted
    literal is well-formed, self-terminating, and reads back as exactly the
    input. This is the injection boundary stated as an invariant instead of as
    a comment.
  - `adocpdf-domain::sandbox` — for any path expression, acceptance depends on
    where the path resolves and on nothing else. Traversal, absolute paths and
    outward links become instances of one rule rather than three test cases.
- **A fuzzing scaffold** (`cargo-fuzz`) over the untrusted-input path, so that
  arbitrary bytes cannot make the renderer panic or fail to terminate. Crashes
  it finds become ordinary regression tests in the normal suite.
- **Mutation testing** (`cargo-mutants`) on a schedule: bugs are injected into
  the source and the suite is asked whether it notices. This is the direct
  answer to "is 90% line coverage worth anything here", and it is enforced
  where it matters rather than reported everywhere and read nowhere.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. `.openspec.yaml` sets `skip_specs: true`, with the reasoning recorded
there: each property tested is already a requirement in `render-first-pdf`'s
`document-rendering` and `project-sandbox` specs. This change strengthens the
evidence, not the requirements.

## Non-goals

- **Lowering the coverage floor.** `AGENTS.md` forbids it, and nothing here is a
  reason to. Mutation testing supplements the floor; it does not buy relief
  from it.
- **Benchmarks.** There is no performance requirement to hold a benchmark
  against, and `README.md` records whether incremental PDF regeneration is
  achievable at all as an open question. A benchmark suite with no target
  measures change without judging it.
- **Snapshot testing of rendered output.** Plausible for a renderer, and a
  separate decision: golden files are a maintenance commitment, and the
  end-to-end tests already read back real PDFs.
- **OSS-Fuzz integration.** Continuous fuzzing at that scale presumes a public
  repository with a maintained response process.
- **Fixing whatever the fuzzer finds.** Findings are reported and turned into
  failing regression tests. Fixing a real bug in the renderer is its own change
  with its own proposal — this one builds the instrument.

## Impact

**Layers touched:** `adocpdf-infra` and `adocpdf-domain` each gain a
dev-dependency on `proptest`. No production dependency changes; no crate's
`[dependencies]` is touched.

**`architecture.toml` must be edited, twice.** The guard reads
`[dev-dependencies]` exactly as it reads `[dependencies]` — deliberately, since
a test that reaches outward breaks the layering just as a library does — so
`proptest` has to be named in the `external` list of both crates or the gate
fails. That is the guard working.

**A new top-level `fuzz/` directory**, outside `crates/`. Its relationship to
the architecture guard, to `deny.toml` and to the workspace is a design
decision, not an incidental detail; see `design.md`.

**Boundaries touched:** the injection boundary (`markup::string_literal`) and
the sandbox boundary (`adocpdf-domain::sandbox`) are both the *subject* of this
change. Neither is modified — new tests are added around them. The determinism
boundary is not touched, with one caveat handled in `design.md`: a property test
that generated its own random seed each run would make the suite
non-reproducible, which the project forbids.

**Toolchain.** `cargo-fuzz` requires a nightly compiler, and
`rust-toolchain.toml` pins stable 1.97.1. Reconciled in `design.md`, not
papered over.

**Cost.** Longer test runs, a new licence to admit to `deny.toml`, and two more
tools to install. Each is argued for individually in `design.md`.
