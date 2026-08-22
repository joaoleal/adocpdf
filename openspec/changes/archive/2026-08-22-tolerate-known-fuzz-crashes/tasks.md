## 1. The record, and the test that keeps it honest

Written before the workflow reads it, so that after this section the suite is
green and the fuzz job is unchanged.

- [x] 1.1 Add `fuzz/known-crashes.toml` with one entry per known upstream panic:
      the inline `image:`/`icon:` macro with no target, and the `%` shorthand
      block attribute list. Each entry carries the input as escaped bytes, the
      upstream location it comes from, and the reason it is tolerated (design
      D1, D3). Verified by the test in 1.2 reading it.
- [x] 1.2 Add an integration test in `adocpdf-infra` that reads the record with
      `std` alone and asserts, for every entry, that the parse boundary refuses
      that input. This is what stops an entry outliving the defect it names.
      Verified by a green `cargo test -p adocpdf-infra`, and by watching it fail
      when an entry's bytes are altered.
- [x] 1.3 Add the companion assertion that the record is not empty and that no
      two entries share a name or bytes — a duplicated entry is a sign the
      record is being edited rather than read. Verified by the test failing on a
      deliberately duplicated entry.

## 2. The job reads it

- [x] 2.1 Add a step to `.github/workflows/fuzz.yml` that reads the run's output
      for every panic location it reported, matches each against the record
      (design D2, amended during implementation), prints each tolerated defect
      with its reason, and fails listing any that are unrecorded. Verified by
      `actionlint` and `zizmor` passing, and by running the script locally
      against the real nightly run that defeated the byte-matching design.
- [x] 2.2 Make the job's success depend on that step rather than on libFuzzer's
      exit status, keeping the artifact upload on failure. Verified by four
      local runs: a known defect passes, a panic in this project's own crates
      fails, a reproducer with no panic location fails, and a clean run passes.
- [x] 2.3 Update the workflow's header comment: it currently tells a reader that
      a failure is not automatically new and to compare by hand. That advice is
      what this change automates, and leaving it would send them to do work the
      job now does. Verified by review.

## 3. Documentation and the gate

- [x] 3.1 Update the fuzzing paragraph in `README.md`, which says every
      reproducer is a permanent test — still true — but not that the job now
      tolerates them by design. Verified by review.
- [x] 3.2 Run `scripts/ci/gate.sh` and hold the 90% coverage floor without
      lowering it. Verified by a green gate across all seventeen jobs.
- [x] 3.3 Run the fuzz target on nightly and confirm the job's logic reports a
      green run naming the defect it tolerated. Verified by a four-minute run
      whose panic the script matched, printed with its reason, and exited zero —
      the same run that showed byte matching could not work.
