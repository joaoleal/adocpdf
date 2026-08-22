## 1. Establish the emission mechanism

- [x] 1.1 Settle design.md D4's unverified assumption: add an emitter test that
      emits a paragraph as concatenated content values
      (`#par(text("a") + strong(text("b")))`) and compiles it through the real
      Typst engine, asserting the rendered text reads `ab` and that `strong`
      took effect. Verified by the test passing; if it fails, D4's fallback is
      taken and the rest of the emitter tasks follow the fallback shape.

## 2. The inline model in `adocpdf-core`

- [x] 2.1 Add `InlineStyle` (strong, emphasis, monospace, superscript,
      subscript, highlight) and `InlineNode` (text, styled span with children,
      hard line break) to `crates/adocpdf-core/src/document.rs`, with rustdoc on
      every public item. Verified by unit tests constructing a nested span and
      asserting its shape, plus `cargo doc` warnings-denied in the gate.
- [x] 2.2 Make `InlineText` a sequence of `InlineNode`, keeping `new(&str)`
      constructing a single text node and adding `plain_text()` for the
      reporting paths. Verified by the existing core test suite continuing to
      pass unchanged, and by a new test asserting `plain_text()` flattens a
      nested span.

## 3. The inline renderer in `adocpdf-infra`

- [x] 3.1 Add the source guard refusing any document containing a Unicode
      noncharacter U+FDD0–U+FDDF, beside the existing control-character refusal
      in `parser.rs`, with a message naming the code point and distinguishing
      itself from a syntax error. Verified by tests covering a refused document,
      the message content, and a document with no noncharacter rendering
      unchanged.
- [x] 3.2 Implement `InlineSubstitutionRenderer` for formatting only —
      `render_quoted_substitution` for the six styles, `render_special_character`
      emitting the HTML entities per design D2 fact 4, `render_line_break` —
      encoding structure with the U+FDD0–U+FDDF markers. Verified by tests
      asserting the encoded string for bold, nested bold-in-italic, monospace,
      superscript, subscript and highlight.
- [x] 3.3 Override every remaining trait method so no inherited HTML can reach
      the output: each emits the text the construct carried and records a
      skipped construct. Verified by a test rendering a document containing a
      link, an image, a footnote, an xref, a callout, an index term, a button, a
      keyboard macro, a menu and an anchor, asserting no `<` survives into the
      rendered text and that every construct is named in the report.
- [x] 3.4 Add the decoder turning the encoded string into an `InlineText` tree,
      converting the HTML entities back last (`&lt;`/`&gt;` before `&amp;`) and
      excluding passthrough regions via `Content::passthroughs()`. Verified by
      round-trip tests including a source `&lt;`, a passthrough containing an
      entity, and a passthrough containing a forged marker.
- [x] 3.5 Switch the mapper to build paragraphs, section headings and the
      document title from the decoder rather than from source spans and
      `doctitle()`. Verified by tests asserting `= *Bold* Title` produces a
      styled title with no markup, and that the heading and paragraph paths
      agree on the same input.
- [x] 3.6 Report unresolved attribute references, at block granularity per
      design D8. Verified by a test asserting an undefined `{attr}` renders as
      written and appears in the report, while a defined one is substituted and
      is not.

## 4. Threading the model through the layers

- [x] 4.1 Carry `InlineText` through `PlanItem` in `adocpdf-domain` and the
      boundary DTOs in `adocpdf-shared`. Verified by the domain use-case tests
      passing against the in-memory fakes, and by the architecture guard.
- [x] 4.2 Emit inline structure from `adocpdf-infra`'s emitter, every text run
      still passing through `markup::string_literal`. Verified by emitter tests
      per style and an end-to-end test reading back the PDF text.
- [x] 4.3 Extend `crates/adocpdf-infra/src/markup.rs`'s `properties` module with
      the claim that no source input can produce a structural marker, and add
      the same claim at lower case count to `tests/injection.rs`. Verified by the
      property tests passing in the gate.
- [x] 4.4 Add `crates/adocpdf-infra/src/<inline decoder>.rs` to the enforced
      file list in `.github/workflows/mutants.yml:57`, beside `markup.rs` and
      `sandbox.rs`. Verified by the mutants workflow running against it.

## 5. Fonts, typography and theming

- [x] 5.1 Embed DejaVuSans-Bold, -Oblique, -BoldOblique and DejaVu Sans Mono
      (regular and bold) in `EmbeddedFonts`, with the Bitstream Vera notice
      already present covering them. Verified by a test asserting the font book
      resolves each requested variant to a distinct face.
- [x] 5.2 Add a monospace family to `Typography`, rejecting a theme naming a
      family the font book cannot provide. Verified by tests for a valid theme,
      a theme naming an unavailable family, and the built-in defaults.
- [x] 5.3 Emit `#set par(linebreaks: "optimized")` from the emitter, so that
      Knuth-Plass breaking runs whether or not text is justified. Verified by an
      emitter test asserting the setting is present, and by a layout test
      rendering a paragraph whose greedy and optimal breaks differ and asserting
      the optimal breaks are the ones produced — a test that fails today.
- [x] 5.4 Add justification, language and widow/orphan costs to `Typography`,
      emitted as `par(justify:)`, `text(lang:)` and `text(costs:)`, defaulting
      to unjustified with no language. Verified by tests for a justified theme,
      a theme naming a language producing hyphenation, an unjustified default,
      and byte-identical output across differing host locales.

## 6. Tier-1 blocks

- [x] 6.1 Add the nesting `PlanItem` grouping variant from design D6. Verified
      by domain tests planning a nested structure and asserting theme
      transitions still resolve correctly across it.
- [x] 6.2 Map and render literal, listing and source blocks verbatim in the
      monospace family, unreflowed. Verified by tests asserting indentation and
      blank lines survive, that no substitution is applied, and that an
      unterminated delimiter does not abort the render.
- [x] 6.3 Map and render the five admonition kinds in both forms, labelled and
      set apart. Verified by tests per kind, in both forms, asserting the label
      appears and multi-block admonitions stay in one region.
- [x] 6.4 Map and render quote and verse blocks with attribution and citation.
      Verified by tests for a quote with both, a quote with neither, and a verse
      preserving its line breaks.
- [x] 6.5 Map and render example blocks, sidebars and open blocks, including
      nesting. Verified by tests for each and for an example containing a
      sidebar.
- [x] 6.6 Map and render thematic and page breaks. Verified by an end-to-end
      test asserting a page break moves content to a later page and a thematic
      break does not.
- [x] 6.7 Map and render block titles. Verified by tests asserting a title
      renders with its block, is distinguishable from a heading, and does not
      affect section nesting.
- [x] 6.8 Ensure comments are dropped without being reported. Verified by a test
      asserting neither a line comment nor a comment block appears in the output
      or in the report.

## 7. Lists

- [x] 7.1 Map and render unordered and ordered lists with nesting and markers.
      Verified by tests asserting nesting depth, indentation and ordered
      numbering.
- [x] 7.2 Map and render description lists. Verified by a test pairing two terms
      with their descriptions.
- [x] 7.3 Honour list continuation. Verified by a test asserting an attached
      paragraph renders inside its item and the list continues afterwards.

## 8. The syntax-support inventory

- [x] 8.1 Write `docs/asciidoc-support.md`: one row per construct with syntax,
      parser support, whether this renderer honours it, and tier — including the
      never-supported rows for audio, video and docinfo, each with its reason.
      Verified by review against the chapter list in `asciidoc-parser`'s
      `src/tests/asciidoc_lang/` and the official syntax quick reference.
- [x] 8.3 Record every construct that tier 1 renders only partially, with what
      is missing and which tier completes it — starting with the footnote,
      whose marker is rendered but whose body is unreachable from the inline
      renderer and is scheduled for tier 3, and the literal HTML entity
      spelling inside a no-substitution passthrough, which the decoder cannot
      tell from an entity the parser produced. Verified by a test asserting every
      row marked partial names a tier later than 1, so a partial row cannot sit
      in the inventory without a commitment to finish it.
- [x] 8.2 Give every row marked as honoured a minimal source sample and add the
      test from design D7 that renders each sample and asserts nothing was
      reported as skipped. Verified by that test passing, and by it failing when
      a row is marked honoured without support.

## 9. Documentation and the gate

- [x] 9.1 Replace `README.md`'s "What does not work yet" prose with a pointer to
      the inventory, and update "What works today". Verified by review and by
      the spelling job.
- [x] 9.2 Update `AGENTS.md`'s "Two rules that are easy to break" to cover the
      marker alphabet and the noncharacter guard, so the next change does not
      reintroduce an in-band forgeable encoding. Verified by review.
- [x] 9.3 Run `scripts/ci/gate.sh` and hold the 90% coverage floor without
      lowering it. Verified by a green gate across all seventeen jobs.
- [x] 9.4 Run the fuzz target on nightly for at least five minutes against the
      new inline path (`cargo +nightly fuzz run parse_plan_emit --
      -max_total_time=300 -timeout=10`), committing any counterexample as a
      regression test. Verified by a clean run or by a committed reproducer.
      Ran well past the five minutes and found two more parser defects: an
      inline `image:`/`icon:` macro with no target panics, and a shorthand
      block attrlist trips a debug assertion. Both reproducers are committed as
      tests in `tests/parser_refusal.rs`, as this project's fuzz workflow
      requires; the guard that contains them is described in D10.
