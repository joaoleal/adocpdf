## Why

The renderer honours four constructs — the document title, section headings,
paragraphs and the `[theme=]` attribute — out of roughly a hundred and twenty
the AsciiDoc language defines. Everything else is reported as skipped. Worse,
the four it does honour are honoured imprecisely: paragraph text is taken from
the **source span**, so `*bold*` reaches the page as three literal characters
and a `{attribute}` reference never resolves at all.

The parser is not what is holding this back. `asciidoc-parser` 0.29.19 carries
a conformance suite that mirrors the official language documentation file by
file — `track_file!("ref/asciidoc-lang/docs/modules/…")` — with 1133
`verifies!` claims spread across all sixteen chapters and no outstanding
to-dos. Tables arrive with columns, spans and alignments already resolved;
lists arrive with their type, marker and checkbox state. Every gap listed below
is a gap in *rendering*, not in parsing.

There is also a defect to correct. `Document::doctitle()` and
`SectionBlock::section_title()` both return **rendered** output, so with the
parser's stock HTML renderer a document titled `= *Bold* Title` puts a literal
`<strong>` on the page today. Paragraphs take the source span and headings take
rendered HTML: the two paths disagree, and both are wrong.

Doing this well needs a decision about where inline structure comes from, and
that decision is now settled by evidence rather than by guesswork — see
`design.md`. In short, `asciidoc-parser` exposes
`Parser::with_inline_substitution_renderer`, a supported extension point that
hands a downstream renderer the inline structure directly. **This does not
re-open the recorded decision to wrap the parser rather than write one.** It is
that decision being honoured: the alternative — scraping the parser's HTML, or
writing an inline parser of our own — is what would have re-opened it.

## What Changes

- **A standalone syntax-support inventory**, `docs/asciidoc-support.md`,
  listing every construct the AsciiDoc language defines against whether the
  parser supports it, whether this renderer renders it, and which tier it is
  scheduled in. It outlives this change: each later tier updates it. It also
  records the constructs that will *never* be supported — audio and video
  blocks, and docinfo — so they stop reading as pending work.
- **An inline model** in `adocpdf-core`. `InlineText` stops being a string and
  becomes a sequence of inline nodes carrying emphasis, strength, monospace,
  superscript, subscript and highlighting. **BREAKING** for the workspace's own
  crates; these crates are not published, so nothing outside the repository
  depends on the shape.
- **Real inline extraction** in `adocpdf-infra`, through a project-owned
  implementation of the parser's `InlineSubstitutionRenderer`. This covers
  bold, italic, monospace, superscript, subscript, highlight, curved quotes,
  character replacements, the typographic apostrophe, hard line breaks,
  attribute references, escaping and passthroughs.
- **Tier-1 blocks**: literal, listing and source blocks; admonitions in both
  their paragraph and delimited forms; quotes and verses with attribution;
  sidebars, examples and open blocks; thematic and page breaks; block titles;
  comments; literal paragraphs; and Markdown-compatible headings and
  blockquotes.
- **Lists**: unordered and ordered with nesting, description lists, and list
  continuation.
- **Font faces to set the above in.** Only DejaVuSans regular is embedded
  today, so there is nothing to render bold, italic or monospace *with*. The
  bold, oblique and bold-oblique faces plus the DejaVu Sans Mono family are
  embedded alongside it, and the theme model gains a monospace family beside
  its body family. All are Bitstream Vera, the licence already recorded in
  `LICENSING.md`; no new licence obligation is created.
- **The line breaker the engine was chosen for, switched on.** Typst selects
  its optimal (Knuth-Plass) breaker only when justification is enabled, and
  defaults justification to off — so every page this renderer has produced so
  far was laid out greedily, by the same class of algorithm as the tool it
  means to improve on. The emitter asks for optimal breaking explicitly, and a
  theme gains justification, language and widow/orphan settings beside its
  families. See `design.md` D9 for the citations.
- **The injection boundary is extended to cover passthroughs.** Passthrough
  content (`pass:[…]`, `+++…+++`) is the one channel that reaches the parser's
  output un-substituted, verbatim. Today that is harmless because everything is
  emitted as a string literal. Once inline structure is carried through the
  same stream it stops being harmless, and the specification has to say so.

## Capabilities

### New Capabilities

- `inline-formatting`: how inline structure within a paragraph, heading or
  title is recognised, and how it is set on the page — including what happens
  to inline constructs that are not yet supported.
- `block-constructs`: the block-level constructs this renderer honours beyond
  the section and the paragraph, and how each is presented.

### Modified Capabilities

- `document-rendering`: the *Supported document constructs* requirement is
  restated against the published inventory rather than an inline list of four
  names, and the *Source content cannot alter rendering instructions*
  requirement gains the passthrough channel.
- `theming`: a theme acquires a monospace family alongside its body family, and
  a theme naming a family no embedded face provides is rejected rather than
  silently falling back. A theme also acquires the settings that govern how
  text is broken into lines — justification, language and widow/orphan
  avoidance — and the renderer commits to breaking paragraphs optimally
  whether or not they are justified.

## Non-goals

- **Tiers two to five.** Cross-references, anchors, footnotes, the table of
  contents, section numbering and special sections (tier 2); tables (tier 3);
  images, icons and the include directive (tier 4); conditionals, STEM, UI
  macros, callouts, source highlighting, counters and index terms (tier 5).
  Each gets its own proposal, scoped against the inventory this change
  publishes.
- **Constructs with no meaning in print.** Audio and video blocks, and docinfo,
  are recorded as permanently unsupported. They are reported as skipped, as
  they are today.
- **Source syntax highlighting.** A source block is set in monospace, with its
  content verbatim. Choosing and embedding a highlighter is tier 5.
- **Honouring passthrough content as markup.** There is no target format to
  pass through *to*. Passthrough content is treated as literal text.
- **A theme file format, incremental rendering, a WASM build, and matching
  asciidoctor-pdf's output.** Unchanged non-goals from `render-first-pdf`;
  nothing here revisits them.
- **Performance work.** No benchmarks and no latency target, consistent with
  `behavioural-testing`. Optimal line breaking costs more than greedy breaking
  by construction; that cost is accepted here rather than measured, since there
  is no target to measure it against.

## Impact

**Layers touched: all but the delivery roots' business logic.**

- `adocpdf-core` — the inline node model; a monospace family on `Typography`,
  together with its justification, language and widow/orphan settings.
- `adocpdf-domain` — plan items carry inline sequences rather than strings; the
  skipped-construct report gains inline locations; new plan items for the
  tier-1 blocks.
- `adocpdf-shared` — boundary DTOs follow the model change.
- `adocpdf-infra` — the parser adapter's mapper; a new inline renderer
  implementing the upstream trait; the emitter, which gains the paragraph and
  text settings above; the embedded font book; the markup escaper's property
  tests.
- `adocpdf-cli` — the skipped-construct report only. No business logic moves.

**Assets.** Four font files are added under
`crates/adocpdf-infra/assets/fonts/`, adding roughly two to three megabytes to
the binary. `deny.toml` needs no change: the licence is already allowed.

**Documentation.** `docs/asciidoc-support.md` is new. `README.md`'s "What does
not work yet" section shrinks to a pointer at it, since prose duplicating a
hundred-row table will drift from it within one change.
