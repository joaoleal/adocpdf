## 1. Establish the rule

- [x] 1.1 Probe every C0, DEL and C1 character against the real parser, alone
      and embedded in text, and record which fail to terminate. *Verified by:*
      the measured table in design.md D1. Result: `U+000B` and `U+000C`, and
      only in a document containing no non-whitespace character.
- [x] 1.2 Confirm the attribution is upstream by calling
      `asciidoc_parser::Parser::parse` directly, with none of this project's
      code involved. *Verified by:* the direct call hangs, so the defect is in
      `asciidoc-parser` 0.29.19 and not in this crate's mapping.

## 2. The guard

- [x] 2.1 Add the check to `AsciidocParser::parse`, before the source reaches
      the parser, refusing exactly the condition established in 1.1 (design
      D1, D4). *Verified by:* the refusal triggers on a whitespace-only
      document containing `U+000B` or `U+000C`, and on nothing else.
- [x] 2.2 Map the refusal to `DomainError::ParseFailed` rather than a new
      variant, naming the character by code point (design D3). *Verified by:*
      the error message contains the code point, and `adocpdf-domain` gains no
      knowledge of the parser or of these characters.
- [x] 2.3 Record in the code the upstream version this works around and the
      condition for deleting it. *Verified by:* the comment names
      `asciidoc-parser` 0.29.19 and says the guard goes when a fixed version is
      adopted.

## 3. Prove it did not break anything

- [x] 3.1 Add tests for the boundary: refused for whitespace-only input with
      `U+000B`/`U+000C`; accepted for the same characters beside real content;
      accepted for empty, spaces-only and newline-only input. *Verified by:*
      all pass, and each asserts behaviour rather than restating the
      implementation.
- [x] 3.2 Confirm the regression tests from `behavioural-testing` now pass
      without being modified. *Verified by:*
      `crates/adocpdf-infra/tests/hang_regressions.rs` is unchanged by this
      change and green.
- [x] 3.3 Confirm no document that rendered before renders differently.
      *Verified by:* the end-to-end suite passes unchanged, and the fixtures
      produce the same bytes.

## 4. Close out

- [x] 4.1 Delete the temporary probe test. *Verified by:*
      `crates/adocpdf-infra/tests/control_probe.rs` is gone; it exists to
      produce a list once, and each hanging case costs a real timeout.
- [x] 4.2 Re-run the fuzz target against the reproducer and confirm it no
      longer hangs through this crate. *Verified by:* the recorded timeout
      artefact is replayed and the target returns.
- [x] 4.3 Run `scripts/ci/gate.sh` and confirm it passes. *Verified by:* the
      gate prints `gate passed`.
