## Why

The weekly `fuzz` workflow now fails on every run, and will keep failing until
`asciidoc-parser` is fixed. `libfuzzer-sys` installs a panic hook that calls
`abort()` *before* unwinding (`src/lib.rs:91`–`94`), so the `catch_unwind` guard
that contains the parser's panics in the shipped renderer never runs under the
fuzzer. The job therefore reports two panics that are already known, already
guarded, and already committed as regression tests — every Sunday, for as long
as the pin stands.

A permanently red scheduled check is worse than no check. It trains whoever sees
it to close the tab, and the one week the job finds something new looks exactly
like the fifty before it. That is the failure this change exists to prevent, not
the panics themselves: those are recorded, contained, and out of our hands.

## What Changes

- **A checked-in record of every crash the fuzzer is expected to find**, one
  entry per known upstream defect, holding the exact input bytes and the reason
  it is tolerated.
- **The fuzz job compares each reproducer it produces against that record** and
  fails only on a crash that is not in it. A tolerated crash is *printed*, with
  its reason, so a green run still says what it found — silence would be the
  same mistake in a quieter form.
- **The same record is read by a test on the pinned stable toolchain**, which
  asserts that every input in it is still refused by the guard. This is what
  stops the record and the guards drifting apart: an entry that upstream fixes,
  or that our guard stops covering, fails the ordinary suite rather than
  lingering as a permanent exemption nobody re-reads.
- Comparison is by **exact input bytes**, never by a message, a stack or a
  filename. A crash that differs by one byte from a known one is a new crash.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. `skip_specs: true`, following `behavioural-testing`, which introduced
fuzzing on the same footing: this changes what CI tolerates and what a test
reads, not what the renderer does. No requirement in any capability describes
the fuzz job, and inventing one to satisfy validation would be worse than
declaring the change specless. The renderer's behaviour on every input in the
record is already required by *Rendering terminates or refuses, on every input*
in `document-rendering`, and is already tested.

## Non-goals

- **Fixing the upstream panics.** They are `asciidoc-parser` 0.29.19's, not
  ours, and the guard already contains them for anyone using the renderer.
- **Making `catch_unwind` work under the fuzzer.** The abort hook is
  `libfuzzer-sys`'s deliberate design — it is how a caught panic still gets
  reported — and defeating it would blind the job to panics in *our* code.
- **Tolerating hangs.** The four non-terminating inputs are refused before
  parsing, so the fuzzer does not reach them; nothing needs an entry.
- **Changing the gate.** Fuzzing is not one of its seventeen jobs and does not
  become one here.

## Impact

**Layers.** None. No crate's source changes: the record is a data file, the
comparison is in the workflow, and the test that reads the record lives in
`adocpdf-infra`'s integration tests beside the guard it checks.

**CI.** `.github/workflows/fuzz.yml` gains a step that inspects
`fuzz/artifacts/` after the run. The job's failure condition narrows from "the
fuzzer found a crash" to "the fuzzer found a crash we have not already
recorded", which is the whole point and also the whole risk.

**Risk, stated plainly.** An allowlist can hide a regression. Three things bound
it: the comparison is over exact bytes, so nothing matches loosely; the record
is asserted against the live guard by a stable-toolchain test, so an entry
cannot outlive the defect it describes; and every tolerated crash is printed
rather than swallowed. If those three stop being true, the job is worse than the
red one it replaced.

**Removal.** Each entry names the upstream defect it stands for. When the pin
moves to a version that fixes one, its entry's test fails — because the input no
longer needs refusing — and the entry goes. That is the intended way for this to
end.
