## 1. Workspace and the guard

- [x] 1.1 Create the Cargo workspace root with a pinned toolchain file (1.97.1,
      with `clippy`, `rustfmt`, and both wasm targets) and workspace-wide lint
      settings. *Verified by:* `cargo metadata` succeeds and `rustup show`
      reports the pinned toolchain.
- [x] 1.2 Create all six crates — `adocpdf-core`, `-domain`, `-shared`,
      `-infra`, `-cli`, `-wasm` — each with a placeholder root module, wired as
      workspace members with no dependencies between them yet. *Verified by:*
      `cargo build --workspace` succeeds.
- [x] 1.3 Add `architecture.toml` declaring each crate's allowed-dependency set
      from the layer table, and the guard test that reads it, resolves each
      crate's intra-workspace dependencies, and fails on an outward edge
      (design D5). Include a case proving the guard actually rejects a
      violation. *Verified by:* `cargo test` passes, and the rejection case
      demonstrates a violation is caught rather than ignored.
- [x] 1.4 Add the quality gate script running `cargo fmt --check`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`, and a `--target wasm32-unknown-unknown` build
      of the crates that must stay wasm-clean. *Verified by:* the script exits
      zero on the current tree.

## 2. Core model

- [x] 2.1 Implement the `Length` value object with validated construction
      (positive, finite) and the unit conversion the theme model needs.
      *Verified by:* unit tests covering valid construction and each rejection
      case.
- [x] 2.2 Implement `PageGeometry` and `Typography` as separately identifiable
      value objects, each rejecting the invalid settings named in the `theming`
      spec (non-positive dimensions, margins leaving no printable area).
      *Verified by:* unit tests asserting each rejection names the offending
      setting.
- [x] 2.3 Implement `Theme` composing geometry and typography, plus
      `ThemeSet` with lookup by identifier and a built-in default. *Verified
      by:* unit tests for lookup hit, lookup miss, and the default's validity.
- [x] 2.4 Implement the transition classifier answering whether a given ordered
      theme pair changes page geometry (design D3). *Verified by:* unit tests
      for geometry-change, typography-only change, and identical-theme
      transitions — the three scenarios in the `theming` spec.
- [x] 2.5 Implement the document model — document, section with nesting level
      and optional theme reference, paragraph, inline text — as the constructs
      the `document-rendering` spec names as supported. *Verified by:* unit
      tests constructing a nested document and asserting its shape.

## 3. Domain: ports, errors, use case

- [x] 3.1 Implement `DomainError` as a typed error enum covering the failure
      modes the three specs require: missing input, parse failure with source
      location, unwritable output, invalid theme, unknown theme, path outside
      root, and non-existent root. *Verified by:* unit tests asserting each
      variant's message reports what its spec scenario requires.
- [x] 3.2 Implement `SandboxedPath` enforcing confinement by resolved location
      rather than spelling, covering traversal segments, absolute paths, and
      outward symlinks, and reporting the effective root without disclosing the
      target. *Verified by:* integration tests against a real temp directory
      for each `project-sandbox` scenario, including a genuine symlink.
- [x] 3.3 Define the domain ports as traits — source store, parser, renderer,
      theme repository, clock — each documenting its `# Errors`. *Verified by:*
      `cargo doc` builds and the guard confirms `adocpdf-domain` depends only
      on `adocpdf-core`.
- [x] 3.4 Implement the theme resolution rule: default applies, a section
      override applies to its subtree, a nested override wins, an unknown theme
      fails. *Verified by:* unit tests against in-memory fakes for each
      `theming` resolution scenario.
- [x] 3.5 Implement the `RenderDocument` use case orchestrating validate →
      read → parse → resolve themes → render → write, returning the report of
      skipped constructs. *Verified by:* unit tests driven entirely by
      hand-written in-memory fakes, asserting both the success path and that
      each failure leaves no output file.

## 4. Boundary DTOs

- [x] 4.1 Implement the render-request and render-report DTOs in
      `adocpdf-shared`, carrying no business rules, plus the mapping to and
      from domain types. *Verified by:* round-trip tests, and the guard
      confirming `adocpdf-shared` depends only on `adocpdf-core`.

## 5. Infrastructure adapters

- [x] 5.1 Choose and vendor the body font, recording its exact licence in
      design.md's dependency table — resolving the item design.md marks
      unverified. *Verified by:* the font file is present, its licence is
      recorded, and it loads into a font book at runtime.
- [x] 5.2 Implement the filesystem source store, reading only through
      `SandboxedPath` and mapping I/O errors to typed domain errors at the
      boundary. *Verified by:* integration tests against a real temp directory
      for read success, missing file, and out-of-root refusal.
- [x] 5.3 Implement the `asciidoc-parser` adapter mapping the parse result into
      the core document model, feeding it the injected reference time, and
      collecting each unsupported construct with its source location rather
      than aborting. *Verified by:* integration tests over real AsciiDoc
      sources — a supported document, a document with an unsupported
      construct, and a malformed one, which still parses (see design D7).
- [x] 5.4 Implement the markup escaper as the single chokepoint of design D2,
      covering every character meaningful to the engine's syntax. *Verified
      by:* unit tests enumerating each metacharacter, plus a test that a
      paragraph of metacharacters lays out identically to one of plain letters.
- [x] 5.5 Implement the emitter turning the core document into engine markup,
      emitting structural directives only from validated model values and all
      source text through the escaper, and emitting a page-scoped directive
      only when the transition classifier says the geometry changed. *Verified
      by:* snapshot tests of emitted markup for a plain document, a
      typography-only transition, and a geometry transition.
- [x] 5.6 Implement the `World` over an in-memory virtual filesystem (design
      D1), serving only sandbox-approved sources, the vendored font, and the
      injected date. *Verified by:* unit tests asserting it resolves a known
      source, returns the injected date, and refuses an unknown path.
- [x] 5.7 Implement the renderer adapter compiling markup through embedded
      Typst and exporting PDF bytes, mapping compilation diagnostics to typed
      domain errors. *Verified by:* an integration test producing non-empty
      PDF bytes beginning with the PDF magic number, and one asserting a
      compilation failure maps to a layout error distinct from a parse error.
- [x] 5.8 Implement the system clock and a fixed clock for tests. *Verified
      by:* a unit test that the fixed clock returns its configured date
      unchanged across calls.

## 6. Delivery

- [x] 6.1 Implement the CLI: input path, output path, optional project root,
      optional theme file; argument parsing only. *Verified by:* tests
      asserting each argument parses and that a missing required argument
      produces a usage error.
- [x] 6.2 Implement the composition root constructing every adapter, injecting
      them into the use case, and mapping domain errors to distinct exit codes
      and messages. *Verified by:* the guard confirming no business logic
      entered `adocpdf-cli`, and tests asserting each failure class maps to its
      own exit code.
- [x] 6.3 Leave `adocpdf-wasm` as a compiling placeholder with no bindgen
      surface, present only so the guard constrains it from the start.
      *Verified by:* it builds for the host and the guard reports no violation.

## 7. End-to-end proof

- [x] 7.1 Add a fixture AsciiDoc document exercising the supported constructs —
      title, nested sections, paragraphs — and a second declaring both a
      typography-only and a geometry-changing section theme. *Verified by:*
      the fixtures parse without error.
- [x] 7.2 Add the end-to-end test rendering the fixture through the real
      binary and asserting a readable PDF is produced whose extracted text
      contains the title and paragraph text. *Verified by:* the test passes
      against a real invocation, not a fake.
- [x] 7.3 Add the determinism test rendering the same fixture twice and
      asserting byte-identical output, with the date supplied by the fixed
      clock. *Verified by:* the test passes on repeated runs.
- [x] 7.4 Add the page-break test asserting a geometry-changing section starts
      a new page while a typography-only change does not. *Verified by:* page
      counts and per-page content of the rendered fixture.

## 8. Documentation

- [x] 8.1 Write `AGENTS.md` as the sole agent context file, carrying over the
      hard constraints from the reference project — no `CLAUDE.md`, no push or
      merge without explicit consent, no skipping tests — alongside this
      project's layer table and conventions. *Verified by:* the file exists,
      and no `CLAUDE.md` is present in the repository.
- [x] 8.2 Write `README.md` covering what `adocpdf` is, how to build it, how to
      run the gate, and the current honest limitations. *Verified by:* a reader
      following it from a clean checkout reaches a rendered PDF.
- [x] 8.3 Record the Apache-2.0 obligation inherited from the engine (design
      D6) in the repository's licensing notes. *Verified by:* the note names
      the engine crates and their licence.
