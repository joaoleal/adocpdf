## Context

See proposal.md — Why. The repository currently contains no code: only
`openspec/config.yaml` and this change. Everything below is therefore
greenfield, and the constraints come from decisions already recorded in the
project context rather than from existing code.

Two of those constraints shape the whole design and are worth restating because
they are counter-intuitive:

- `typst::compile` consumes **source markup, not an intermediate
  representation**. There is no typed AST we can hand it. So the boundary
  between our model and the engine is a string, which makes emission an
  injection boundary rather than a data mapping. [verified]
- Typst's `World` trait (`source`, `file`, `font`, `today`) is the engine's own
  port for the outside world. It is Typst's inversion of control, not ours, and
  it belongs in infra. [verified]

Verified toolchain state on this machine: `rustc` and `cargo` 1.97.1, with
`wasm32-unknown-unknown` and `wasm32-wasip1` installed. This clears every MSRV
listed below.

## Goals / Non-Goals

**Goals:**

- Prove the AsciiDoc → model → markup → engine → PDF path holds end to end,
  through all six crates, with the dependency rule machine-enforced from the
  first commit.
- Establish the seams later changes widen: the parser port, the renderer port,
  the theme resolution rule, the sandbox, and the clock.
- Make the architecture falsifiable now, while reversing it is still cheap.

**Non-Goals (design-level; see proposal.md for scope non-goals):**

- No abstraction over the *choice* of layout engine. The renderer port hides
  Typst from the domain, but the port is shaped for a markup-consuming,
  whole-document engine. A future engine with a different contract would need a
  new port, and that is an acceptable cost to avoid speculative generality.
- No caching or memoization layer of our own. Typst carries `comemo`
  internally; adding a second cache before measuring would be guesswork.
- No plugin or extension mechanism.

## Decisions

### D1. `World` is implemented in infra, over an in-memory virtual filesystem

**Decision.** `adocpdf-infra` owns the `World` implementation. It is
constructed per render from an in-memory map of virtual paths to sources, plus
an embedded font set, plus a date value passed in by the caller.

**Why.** `World` is Typst's port, and letting it reach the real filesystem
directly would put uncontrolled I/O below our sandbox — a document could then
reach a file our own path validation never saw. Serving `World` from memory
means every byte it can read passed through the sandbox first, which is what
`project-sandbox` requires. It also makes `World` trivially usable under WASM,
where there may be no filesystem at all.

**Alternative rejected.** A `World` that reads the disk lazily. Simpler to
write, and it is what Typst's own CLI does, but it puts file access outside our
control and would make the sandbox requirements unenforceable at the only layer
that can see them.

**Dependency direction.** `adocpdf-infra` → `typst`. The domain never names
`World`; it sees only its own renderer port. The guard sees this as `typst`
appearing in `adocpdf-infra`'s manifest and nowhere else, and will fail if
`typst` appears in `core`, `domain`, or `shared`.

### D2. Emission produces Typst markup through a single escaping chokepoint

**Decision.** All source-derived text reaches the output through one escaping
function. Emission never concatenates parsed text into markup directly.

**Why.** This is the injection boundary named in the project context, and it is
what `document-rendering`'s "source content cannot alter rendering
instructions" requirement rests on. A single chokepoint is testable — the tests
can enumerate the engine's metacharacters against one function — whereas
escaping scattered across emitters cannot be verified by inspection.

**Approach.** Content text is emitted so that no character is interpreted as
markup. Structural constructs (headings, page geometry, typography) are emitted
only from *our own* model values, never from source strings. Numeric values are
formatted from validated value objects, so the alphabet reaching the output for
structural positions is closed.

**Trade-off.** Escaping everything means an author cannot pass raw Typst
through an AsciiDoc document. That is deliberate for now; a passthrough
construct would be a new capability with its own threat model.

### D3. Theme transitions are classified before emission, not during

**Decision.** The core model exposes, for any ordered pair of themes, whether
the transition changes page geometry. Emission asks that question and emits a
page-scoped directive only when the answer is yes.

**Why.** The engine's behaviour — a mid-document `set page` starts a new page
[verified] — is a property of the engine, but the *specified* behaviour is
ours: geometry changes break the page, typography changes do not. Deriving the
classification in core makes it unit-testable without invoking the engine at
all, and it satisfies the `theming` requirement that the system can report
whether a transition breaks the page before rendering.

**Alternative rejected.** Emitting `set page` at every theme boundary and
letting the engine decide. This would break the page on typography-only
changes, contradicting the spec.

### D4. The clock is a port; the date is a value

**Decision.** A `Clock` port in `adocpdf-domain` supplies the date. Infra
provides a system implementation; tests and reproducible builds use a fixed
one. The date flows into both `World::today` and the parser's reference time.

**Why.** Both the engine and the parser will otherwise read the host clock,
which breaks the determinism requirement. Injecting one value into both keeps
them consistent — a document that shows a date and one that resolves a
timestamp attribute agree.

**Note.** `asciidoc-parser` exposes `ReferenceTime`, and `World` requires
`today`. That both accept an injected value is why determinism is achievable at
all. [verified that both APIs exist; not yet verified by running them]

### D5. The architecture guard is a test, not a script

**Decision.** The guard is a test in the workspace that reads each crate's
manifest, resolves its intra-workspace dependencies, and asserts they fall
within the allowed set from the layer table. It fails the build on violation.

**Why.** `cargo test` already runs everywhere — locally, in the gate, in CI —
so the guard cannot be forgotten or skipped the way a separate script can. It
also means a violation is reported with the same ergonomics as any other test
failure.

**Data source.** The allowed sets live in a checked-in `architecture.toml` so
the rule is stated once, in data, rather than duplicated across manifests and
prose.

**Limitation.** Manifest-level checking catches crate-to-crate violations, not
a layering mistake made *within* a crate. That is the right granularity here
because the layers are crates, but it should be stated rather than assumed.

### D6. Third-party dependencies, verified

Versions and licences confirmed against the crates.io API on 2026-08-16:

| Crate | Version | Licence | MSRV | Confined to |
|---|---|---|---|---|
| `typst` | 0.15.1 | **Apache-2.0** | 1.92 | `adocpdf-infra` |
| `typst-pdf` | 0.15.1 | **Apache-2.0** | 1.92 | `adocpdf-infra` |
| `asciidoc-parser` | 0.29.19 | MIT OR Apache-2.0 | 1.88.0 | `adocpdf-infra` |
| `clap` | 4.6.6 | MIT OR Apache-2.0 | 1.85 | `adocpdf-cli` |
| `typst-layout` | 0.15.1 | **Apache-2.0** | 1.92 | `adocpdf-infra` |
| `thiserror` | 2.0.20 | MIT OR Apache-2.0 | 1.71 | `domain`, `infra`, `cli` |
| `toml` | 1.1.4 | MIT OR Apache-2.0 | 1.85 | `xtask` (tooling, not a layer) |
| `pdf-extract` | 0.12.0 | MIT | — | `adocpdf-cli`, tests only |

`typst-layout` was not foreseen: `PagedDocument` — the type `typst::compile`
returns and `typst_pdf::pdf` consumes — lives there and is not re-exported
through `typst`. `pdf-extract` is test-only, so that the end-to-end tests assert
on the finished PDF rather than on a byte buffer this code already held. Each
addition required an explicit edit to `architecture.toml`, which is the guard
working as intended.

**Correction, made during implementation.** An earlier draft of this table
listed `thiserror` as confined to "core, domain". That contradicted the project
context, which states `adocpdf-core -> (nothing; std only)`. The context is the
controlling value, so `adocpdf-core` takes no dependency at all: its validation
errors carry hand-written `Display` and `Error` implementations. The cost is a
few lines per error type; the benefit is that the innermost ring genuinely has
an empty dependency set, which is the property the layer table promises.

**Licence note.** `typst` and `typst-pdf` are Apache-2.0 **only** — not the
usual MIT/Apache dual licence the rest of the tree uses. Anyone redistributing
`adocpdf` inherits Apache-2.0 obligations from the engine. This is a fact to
record, not a blocker, but it should be a conscious choice rather than a
discovery made later.

**Font, resolved during implementation.** The table above originally omitted a
font, marked unverified pending a choice. It is now settled:

| Asset | Version | Licence | Confined to |
|---|---|---|---|
| DejaVu Sans (`DejaVuSans.ttf`) | packaged 2.37 | **Bitstream Vera** | `adocpdf-infra/assets/fonts` |

Chosen because it is permissively licensed, has wide Unicode coverage, and was
already present on the build machine so its bytes could be verified rather than
downloaded on trust. The licence requires the permission notice to travel with
every copy of the font software, so `LICENSE-DejaVu.txt` sits beside the `.ttf`
and must be distributed with the binary.

The font is **embedded in the binary**, not read from the host. A renderer that
searched the system for fonts would produce different output on different
machines — breaking determinism — and would find nothing at all under WASM. The
cost is roughly 760 KB of binary size and a fixed repertoire; widening the
repertoire later is additive.

### D7. AsciiDoc parsing is total, so there is no parse-failure path

**Discovered during implementation.** `asciidoc-parser::Parser::parse` returns a
`Document`, not a `Result`. That is not an omission in the crate: AsciiDoc has
no parse errors. Every byte sequence is a valid AsciiDoc document, and Ruby
Asciidoctor behaves the same way — malformed input degrades into paragraphs and
literal text rather than being rejected. [verified against the crate's API and
its documented behaviour]

**Consequence.** The `document-rendering` spec originally required a failure
when "the source is not valid AsciiDoc". That state cannot be reached, so the
requirement could never be satisfied *or* violated. It has been reworded to
describe what actually happens: malformed source still produces a document, and
anything unrepresentable is reported as skipped.

**Decision.** `DomainError::ParseFailed` is kept in the error enum even though
no current adapter produces it. The port's contract — "parsing may fail" — is
about the *port*, not about today's implementation, and a stricter front end or
a different source language would need it. The alternative, removing it, would
mean a breaking change to the error type the first time a parser can fail.

**Cost.** One unconstructible variant, and a reader who may wonder why. The
rustdoc says why.

## Risks / Trade-offs

- **The Typst dependency tree may not build for WASM.** → Not mitigated,
  scoped out. `adocpdf-wasm` is created empty in this change precisely so that
  discovering this later costs a crate's worth of rework rather than a
  redesign. The layer rules already forbid the domain from depending on the
  engine, which is what keeps the option open.

- **Emitting markup is inherently a stringly-typed boundary.** → The single
  escaping chokepoint (D2) plus a closed alphabet for structural positions.
  Accepted rather than solved: the engine's API leaves no alternative.

- **The renderer port may be shaped too closely to Typst.** → Accepted. See
  Non-Goals. Recorded so a future engine change is understood as port surgery,
  not a swap.

- **Determinism can be broken silently.** → A test that renders the same input
  twice and compares bytes. It will catch clock leakage and iteration-order
  nondeterminism, which are the two plausible sources. It will not catch
  nondeterminism inside the engine itself; that is unverified.

- **Skipping unsupported constructs could hide real data loss.** → The spec
  requires reporting each skipped construct with its location, so omission is
  never silent. The trade-off is deliberate: aborting on the first unsupported
  construct would make the skeleton useless against any real document.

- **`architecture.toml` can drift from the layer table in `config.yaml`.** →
  Both are checked in and reviewed together; the guard test reads the former.
  Not fully mitigated — two sources of truth remain, and collapsing them is
  worth revisiting if they diverge once.

## Open Questions

These are deferrable: none changes the specs, the approach, or the task
breakdown of *this* change.

- **What is the correctness oracle?** asciidoctor-pdf cannot be it — the goal
  is to lay out better, so disagreement is expected rather than diagnostic.
  Candidates are golden-file snapshots of our own output plus targeted
  assertions on extracted text and page counts. This change uses the latter;
  choosing a long-term oracle can wait until there is enough output to compare.

- **Is partial PDF regeneration achievable at all?** PDF is a page-structured
  container, so regenerating only visible content may require SVG preview for
  the interactive path and full PDF only on export. Answering it needs
  measurement against a working renderer, which is what this change produces.

- **Does the Typst tree build for `wasm32-wasip1`?** Both targets are
  installed, so this is answerable in minutes once the workspace exists — but
  answering it changes nothing here, since no WASM surface is built.
