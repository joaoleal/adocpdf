## Why

The project has decisions but no code. The engine bet (embed Typst), the front
end (wrap `asciidoc-parser`), and the layer rules are all recorded, yet nothing
proves they compose. Every one of the four differentiators — progressive
updates, better line breaking, per-section themes, WASM/WASI — depends on the
same claim: that AsciiDoc can be driven through Typst as a library, in-process,
under a strict inward dependency rule.

That claim is unproven. `typst::compile` takes source markup rather than an IR,
so the whole design rests on generating Typst markup from a parsed AsciiDoc
tree and getting a `PagedDocument` back. If that seam does not hold, the layer
table is wrong and the later differentiators are built on sand.

So the first change is a walking skeleton: the thinnest possible path that
touches every layer and produces a real PDF from a real `.adoc` file. It exists
to make the architecture falsifiable early, while reversing it is still cheap.

## What Changes

A Cargo workspace is created with the six crates the layer table names, plus a
checked-in guard that fails the build when a dependency points outward. The
guard is part of this change, not a follow-up: a dependency rule that is not
machine-enforced is a comment.

On top of the scaffold, one end-to-end path is implemented:

- The CLI accepts an input `.adoc` path and an output `.pdf` path.
- The input path is validated against a sandbox root before any read.
- The file is read through a source-store port and parsed through a parser
  port backed by `asciidoc-parser`.
- The result is mapped into the core document model — enough of it to carry a
  document title, sections, paragraphs and inline text.
- A theme is resolved for each section, distinguishing themes that change page
  geometry (which force a page break) from themes that change only typography
  (which do not).
- The document is emitted as Typst markup, with everything interpolated from
  the parsed tree escaped at the boundary.
- Embedded Typst compiles the markup through a `World` implemented in infra,
  with the date supplied by an injected clock so output is reproducible.
- The resulting PDF bytes are written to the output path.

Each step exists as a seam that later changes widen. None of them is complete
AsciiDoc, complete theming, or fast.

## Capabilities

### New Capabilities

- `document-rendering`: turning an AsciiDoc source into PDF bytes — the parse,
  map, emit, compile and write path, its failure modes, and the determinism
  guarantee that identical inputs produce byte-identical output.
- `theming`: resolving which theme applies to which section, and the rule that
  distinguishes a page-geometry theme change (forces a page break) from a
  typography-only change (does not).
- `project-sandbox`: constraining which paths may be read or written, so that
  neither a CLI argument nor an AsciiDoc include directive can reach outside a
  declared root.

### Modified Capabilities

None. This is the first change in the project; no specs exist yet.

## Non-goals

- **Progressive/incremental rendering.** The skeleton compiles the whole
  document every time. Whether partial *PDF* regeneration is achievable at all
  — as opposed to SVG preview of visible pages — is an open question this
  change deliberately does not answer.
- **Complete AsciiDoc coverage.** Only the constructs the walking skeleton
  needs. Tables, includes, admonitions, cross-references, callouts and
  attribute substitution are out.
- **A theme file format.** Themes exist as a model with a per-section
  resolution rule. Authoring themes in a user-facing file is a later change.
- **A WASM build target.** `adocpdf-wasm` is created as a crate and kept
  inside the dependency rules, but no `wasm-bindgen` surface is implemented and
  no WASI host is wired. Whether the Typst dependency tree builds for
  `wasm32-wasip1` is untested and stays untested here.
- **Matching asciidoctor-pdf output.** The stated goal is to lay out *better*,
  so asciidoctor-pdf cannot serve as the correctness oracle. What replaces it
  is an open question recorded in design.md, not settled here.
- **Performance work.** No benchmarks, no caching strategy, no measured
  latency target.

## Impact

**Layers touched — all of them.** That is the point of a walking skeleton:

- `adocpdf-core`: the document and theme model. No dependencies.
- `adocpdf-domain`: the render use case, the ports it calls, and
  `DomainError`.
- `adocpdf-shared`: the boundary DTO for a render request.
- `adocpdf-infra`: the filesystem source store, the `asciidoc-parser` adapter,
  the Typst `World` and compiler adapter, the system clock.
- `adocpdf-cli`: argument parsing and the composition root.
- `adocpdf-wasm`: created, empty, and constrained by the guard.

**New third-party dependencies.** `typst` and `asciidoc-parser` enter the tree,
both confined to `adocpdf-infra`. Exact versions and licences are recorded in
design.md.

**Tooling.** A Cargo workspace, a pinned toolchain, the architecture guard, and
a quality gate wiring `cargo fmt --check`, `cargo clippy -- -D warnings`,
`cargo test` and the guard together.

**Decisions re-opened.** None. This change implements the decisions already
recorded in the project context rather than revisiting them. The open questions
it surfaces — the correctness oracle, the feasibility of partial PDF
regeneration, and whether Typst builds for `wasm32-wasip1` — are recorded as
non-goals and carried into design.md rather than answered here.
