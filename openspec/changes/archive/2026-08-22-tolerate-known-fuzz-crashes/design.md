## Context

See `proposal.md` — Why. Two facts shape everything below, both verified against
the installed dependency rather than assumed:

1. **`libfuzzer-sys` aborts before unwinding.** `fuzz_target!` takes the current
   panic hook and installs one that calls the old hook and then
   `std::process::abort()` (`libfuzzer-sys-0.4.13/src/lib.rs:91`–`94`), with a
   comment saying it is deliberate so that the fuzzer sees the crash. So
   `catch_unwind` in `parser.rs` cannot run under the fuzzer, and no change on
   our side of the boundary will make it.
2. **Two crashes are reproducible today**, both recorded as tests in
   `crates/adocpdf-infra/tests/parser_refusal.rs`: an inline `image:`/`icon:`
   macro with no target (`content/macros.rs:291` indexes an optional regex
   group), and a block attribute list combining `%` shorthands
   (`attributes/element_attribute.rs:509`, a `debug_assert!`).

The four non-terminating inputs need no entry: they are refused before parsing,
so the fuzzer never reaches a hang.

## Goals / Non-Goals

**Goals:**

- A green fuzz run means "nothing new", and says out loud what it tolerated.
- The record cannot outlive the defects it describes without a test failing.
- A new crash is still a failure, on the first run that finds it.

**Non-Goals:**

- Deduplicating crashes by similarity. Exact bytes only; see D2.
- Making the fuzz job part of `scripts/ci/gate.sh`. It still needs nightly and
  still takes longer than a merge should wait.

## Decisions

### D1 — The record is a data file, read by both the workflow and a test

`fuzz/known-crashes.toml` holds one entry per known defect: a name, the input as
an escaped byte string, the upstream location it comes from, and the reason it
is tolerated.

The alternative was to derive the list from the constants already in
`parser_refusal.rs`. Rejected: it means parsing Rust source from a shell script,
which is fragile in the way that matters — it fails *open*, tolerating
everything, on the day someone reformats the test file.

A data file both sides read is what keeps them honest. The workflow compares
artifacts against it; a stable-toolchain test asserts that every entry is still
refused by the guard. Neither side can quietly drift: an entry upstream has
fixed makes the test fail, and a guard that stops covering an entry does too.

**Dependency direction.** Nothing new crosses a layer. The record is data under
`fuzz/`, and the test that reads it is an integration test of `adocpdf-infra`,
which already owns the guard. `architecture.toml` is untouched, and the
architecture guard sees no change — the correct outcome, not an oversight.

**Third-party crates.** The test needs to read TOML. `toml` is already in the
workspace lock as a transitive dependency, but adding it as a direct
`dev-dependency` would still be an addition and would need an
`architecture.toml` entry. D3 avoids it entirely.

### D2 — Matching is by panic location, not by input bytes

An entry names a **defect**, and a run is tolerated when every panic it reported
happened at a recorded location.

**This decision was reversed during implementation, by evidence.** It first said
exact input bytes: a crash matched when the artifact's bytes equalled the
entry's. That was implemented, tested, and then a four-minute nightly run
produced a fifty-byte input reaching `element_attribute.rs:509` — a defect
already recorded, by an input that was not. Byte matching tolerates an *input*,
and the set of inputs that reaches one defect is unbounded, so the job would
have gone red again on the first run after every update to the record. It would
have failed at exactly the thing this change exists to fix.

The panic location is the stable identity. A new defect panics somewhere new; a
panic anywhere in this project's own crates is not recorded and fails, which is
the property that matters most.

**What is given up.** Two different upstream defects on the same line would be
indistinguishable. That is acceptable — the entry's reason describes what is
known, and a line that starts panicking for a second reason is a change in
upstream, not a regression here. Byte matching's precision bought nothing
against that and cost the change its purpose.

### D3 — The record's format is chosen so the test needs no parser

Each entry is a line of the form `name = "escaped bytes"` under a comment block
giving the upstream location and the reason. The test reads it with the same
`std`-only splitting the workflow uses, so neither side gains a dependency and
the two cannot disagree about what the file says.

Escaping is TOML's own: `\n`, `\t`, `\r`, `\\`, and `\uNNNN` for anything else
outside printable ASCII.

**Amended during implementation.** This decision first said `\xNN`, which is
what `printf %b` reads directly. It is not valid TOML, and the gate's `taplo`
job parses every `.toml` in the repository — so the file failed the gate on the
first run. Using TOML's escapes keeps the file genuinely TOML, which is worth
more than the shell's convenience: it is checked by a tool on every gate run
rather than only by the two readers that consume it.

The shell converts `\u00NN` to the `\xNN` that `printf` understands, and
**refuses** anything above U+00FF rather than passing it through. Without that
refusal the two readers would silently disagree about the same file, which is
the one thing this format exists to prevent.

**Alternative considered.** One file per crash under `fuzz/known-crashes/`, byte
for byte, no escaping at all. It is simpler to compare and needs no format,
but a reviewer cannot see what a binary file contains in a diff, and the reason
each is tolerated would have to live somewhere else — which is where an
allowlist starts rotting.

### D4 — The job prints what it tolerated, and fails on anything else

After the fuzzer exits, the step reads the run's output for every
`panicked at <path>:<line>:<col>` it reported, matches each against the record,
prints the ones it tolerated with their reasons, and fails listing any that are
unrecorded. A run that tolerates something still writes it to the log, so
"green" never means "silent".

A crash that reports **no** panic location fails: that is what a timeout or an
out-of-memory looks like, and neither is recorded. The four non-terminating
inputs are refused before parsing, so a timeout appearing at all would be a new
finding.

The artifacts are uploaded on failure as they are today. That step's condition
becomes the new failure condition rather than libFuzzer's exit status.

**Boundaries.** None of injection, sandbox or determinism is touched: no
document content reaches the record, nothing here reads a path from a document,
and the comparison depends on nothing but bytes on disk. The fuzz target itself
is unchanged.

## Risks / Trade-offs

- **An allowlist can hide a regression** → bounded three ways, all of which must
  hold or the job is worse than red: only recorded locations are tolerated and
  a panic in our own crates never is, a stable-toolchain test asserts every
  entry is still guarded, and every tolerated panic is printed.
  If a future change loosens any of the three, this decision should be revisited
  rather than extended.
- **A second, unrelated defect on a recorded line would be tolerated** → judged
  acceptable in D2, and visible anyway: the panic message in the log differs
  even when the line does not.
- **The record could grow into a graveyard** → each entry names the upstream
  defect it stands for, and its test fails once that defect is fixed, so the
  entry has a defined end. Nothing prunes it automatically.
- **Escaped bytes in TOML-ish text are easy to get subtly wrong** → the test
  reads the same file the workflow does, so a wrongly escaped entry fails the
  ordinary suite rather than silently matching nothing.

## Migration Plan

Not applicable — CI configuration and a data file. The order is: write the
record and the test that checks it against the guard, then teach the workflow to
read it. After the first step the suite is green and the job is unchanged; after
the second the job is green for the right reason.

## Open Questions

None.
