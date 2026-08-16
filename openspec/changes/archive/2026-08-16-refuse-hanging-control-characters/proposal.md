## Why

Fuzzing found a one-byte denial of service.

`asciidoc-parser` 0.29.19 does not return when given a document consisting of a
single form feed, `U+000C`. Not slowly — at all. The fuzz target found it within
seconds of first running, libFuzzer minimised it to one byte, and a direct call
to `asciidoc_parser::Parser::parse("\u{c}")` with none of this project's code
involved reproduces it. Left alone for 169 seconds it was still spinning at 99%
of a core.

`SECURITY.md` already says what this is:

> A panic or a hang on untrusted input is a vulnerability, not a rendering bug.

So this is not a performance question or a compatibility question. `adocpdf`
accepts a document and is expected to terminate, and today one byte prevents
that. Anything that renders a document it did not write — a service, a CI job, a
batch conversion — can be stopped by a file containing a single character.

The defect is upstream and its fix is not ours to make. What is ours is that we
hand the input over.

## What Changes

- **`AsciidocParser` refuses input containing a control character that upstream
  cannot process, before that input reaches the parser.** The document is
  rejected with a typed domain error naming the offending character and where it
  was found. It is not silently stripped: a document that would have hung is a
  document whose author needs telling, and quietly changing someone's content is
  worse than declining it.
- **The regression tests that `behavioural-testing` committed start passing**,
  because the hang is no longer reachable through this crate.
- **The refusal is a documented behaviour of the parse boundary**, not an
  implementation detail — it changes what a caller observes for a class of
  input, so it belongs in the spec.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document-rendering` — the parse boundary gains a refusal for input that
  cannot be parsed safely.

## Non-goals

- **Fixing `asciidoc-parser`.** The defect is upstream. Reporting it there is
  worth doing and is not this change; this change stops the project handing it
  input it cannot survive.
- **A general input sanitiser.** This is not an opportunity to start
  normalising, transliterating or repairing documents. The scope is exactly the
  control characters that are demonstrated to prevent termination.
- **Guessing.** Only characters actually shown to hang are refused. A guard
  built from suspicion would refuse valid documents and would still be a guess.
  The set is established by measurement and the measurement is a task.
- **Changing what a well-formed document renders to.** No document that
  currently renders may render differently, and that is a task with a test.

## Ordering

Depends on `behavioural-testing`, which produced the finding, the reproducer at
`fuzz/artifacts/parse_plan_emit/timeout-1e32e3c3…`, and the failing regression
tests in `crates/adocpdf-infra/tests/hang_regressions.rs` that this change turns
green.

## Impact

**One layer.** `adocpdf-infra`'s parser adapter. The domain gains no new rule:
this is an adapter refusing input its external technology cannot handle, which
is exactly what the boundary is for, and it maps to an existing `DomainError`
variant rather than a new concept.

**A behaviour change at the boundary.** Input that previously hung now returns
an error. No input that previously succeeded may change, which is the thing to
verify rather than assert.

**The gate goes green again.** Two tests committed by `behavioural-testing`
currently fail, documenting the hang. They are not modified by this change —
they pass because the defect they describe is no longer reachable.
