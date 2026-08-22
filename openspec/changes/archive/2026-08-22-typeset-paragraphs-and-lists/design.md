## Context

See `proposal.md` — Why. What matters here is how the three defects are
produced, all of them in `adocpdf-infra`'s emitter or the inline decoder feeding
it, and all of them invisible to the tests because the tests read markup.

Three facts were verified against the installed dependencies rather than assumed,
and the design rests on them:

1. **A newline inside a Typst string is a line break.** Compiling `#par("a\nb")`
   and `#par("a b")` and reading the laid-out frames gives `"ab"` and `"a b"` —
   the first has no space because the newline became a break, not a space.
2. **The engine's list elements exist and take explicit content.**
   `ListItem { body }` (`typst-library-0.15.1/src/model/list.rs:168`),
   `EnumItem { number: Smart<u64>, body }` (`src/model/enum.rs:247`–`254`, the
   number a *positional* argument), and `TermItem { term, description }`
   (`src/model/terms.rs:130`–`137`).
3. **The engine sets every heading below level 2 at the same size.** Its
   show-set scales text by `1.4` at level 1, `1.2` at level 2, and `1.0` for
   everything else (`src/model/heading.rs:281`–`285`). Levels 3, 4 and 5 are
   identical because the engine makes them so, and the renderer never says
   otherwise.

One constraint shapes the first decision more than anything else: **verse blocks
currently depend on the defect.** `emitter.rs` sets a tighter leading for a verse
and then emits its content the same way as any paragraph — the line breaks
survive only because the newlines in the string are breaks. Its comment says
"the engine must not fill lines for it", and nothing makes that true. Collapsing
newlines without addressing verse would silently reflow every verse in every
document, and the existing "A verse keeps its line breaks" test asserts on
markup, so it would still pass.

## Goals / Non-Goals

**Goals:**

- The model carries no line break that the author did not ask for, so a
  consumer other than this emitter — a future WASM renderer, an HTML preview —
  does not have to repeat the same collapse.
- List presentation is delegated to the engine's own elements, so marker
  placement, hanging indent and nested indentation come from one implementation
  rather than from arithmetic in the emitter.
- The suite gains a way to assert about a laid-out page. Every claim in this
  change is about geometry, and none of them can be tested any other way.

**Non-Goals:**

- Heading sizes as a theme setting. The requirement is that levels differ; a
  scale computed by the emitter satisfies it without a change to
  `adocpdf-core`'s model, which keeps this change inside one crate. D3 records
  what a later change would do instead.
- Replacing the markup-level tests. They are cheap and they localise a failure
  to the emitter; the render-level tests answer a different question.
- Deciding a description list's typographic form. The previous change left it
  open deliberately, and switching to the engine's `terms` element is the
  smallest answer rather than the final one.

## Decisions

### D1 — Soft newlines collapse in the inline decoder, and verse gains real breaks

A newline inside `InlineNode::Text` becomes a single space as the inline tree is
built, together with any horizontal whitespace either side of it, so that a
source line ending in a space does not produce two. `InlineNode::LineBreak` is
untouched: it is how a hard break is already represented, and the model already
distinguishes the two.

Verse is handled where the distinction is known. The mapper knows a block is a
verse; it converts that block's line endings into `InlineNode::LineBreak` nodes
rather than leaving them as newlines in text. A verse's line breaks *are* hard
breaks — that is what a verse means — so this makes the model say what it means
instead of relying on a string-encoding accident.

**Alternatives considered.** Collapsing in the emitter, at `string_literal`:
rejected because the model would keep carrying newlines that mean nothing, and
every future consumer would have to know to ignore them. Leaving the collapse
to the engine: not possible — fact 1 above is not configurable, a string's
newline is a break.

**Boundary.** This does not touch the injection boundary: text still reaches the
page through `string_literal`, and the alphabet of markers is unchanged. It does
touch determinism only in the sense that output changes for every wrapped
document, which is the point of the change.

### D2 — Lists are emitted as the engine's list, enum and terms elements

Each item's content is passed as an argument to `list.item`, `enum.item` or
`terms.item` rather than being written after a marker the emitter spells out.
An ordered item passes its number positionally, so the number on the page is the
one the renderer determined, which is what the `block-constructs` delta now
requires.

This fixes more than the reported defect. The current hand-built
`#block(inset: …)` gives no hanging indent, so an item long enough to wrap sets
its second line under the marker rather than under the text — a defect the proof
sheet did not happen to show because every sample item was short. It also ends
the one place where emitter-generated text is read back as markup: `1. ` is
currently re-parsed by the engine as its own enumeration syntax, which is how
ordered items came to look correct while bullets did not.

**Alternatives considered.** Keeping the hand-built blocks and dropping the
`#par` wrapper so the marker and the text share a paragraph: it fixes the
reported symptom in the fewest lines, and leaves the hanging indent wrong and
the marker still written as markup. Building each item as a two-column `grid`:
full control, but it re-implements what the engine already does and would have
to grow its own rules for nesting and for the description form.

**Trade-off, stated.** The engine's `terms` element presents a description list
as a run-in term rather than the bold term on its own line the emitter produces
today. The `block-constructs` requirement only asks that each term renders with
its description, so both satisfy it; this is a presentation change and the
change should say so rather than let it arrive unannounced.

### D3 — Heading sizes are computed by the emitter, from the theme's body size

The emitter emits a size for each heading level it honours, derived from the
theme's body size, so that no two levels are set identically. It does not read
anything from the environment, and the scale is a property of the code rather
than of the host, so determinism is unaffected.

**Alternatives considered.** Adding a per-level size to `Typography` in
`adocpdf-core`: the honest long-term answer, since heading scale is a theme
decision and an authored theme will eventually want it. Rejected *for this
change* because it widens a layout fix into a model change across four crates,
and the requirement — that levels be distinguishable — is met without it. When
the theme file arrives, this is the first thing it should take over.

### D4 — A shared render-level test helper, built before the fixes

A test-only module renders a fixture through the real path and exposes what the
page actually looks like: each text run with its position, font family and size.
That is enough to ask the questions this change is about — do the marker and the
first word of an item share a baseline, does a paragraph's line count change
when the measure changes, do heading sizes strictly decrease.

It extends what `tests/blocks.rs` already does. That file's `rendered` module
already compiles a document and walks the laid-out frames; the helper generalises
it from "the text on each page" to "where each run sits and how it is set", and
is shared by the tests that need it rather than copied.

It is built **first**, before any of the three fixes, so that each fix lands
against a test that fails for the right reason beforehand. Every defect in this
change existed because the instrument was missing; building it last would prove
nothing.

**Dependency direction.** Nothing new. The helper lives in `adocpdf-infra`'s
integration tests, uses `typst` and `typst-layout`, which that crate already
depends on, and adds no entry to `architecture.toml` — so the architecture guard
sees no change, which is the correct outcome rather than an oversight.

## Risks / Trade-offs

- **Collapsing newlines touches every paragraph in every document** → it is the
  widest-reaching edit here, which is why D4 is built first and why the
  `inline-formatting` delta asks for the same paragraph written two ways to
  render identically. That test fails today and cannot pass by accident.
- **Verse silently reflows if the mapper misses a case** → the existing verse
  test asserts on markup and would not notice. It is rewritten as a
  render-level test in this change, asserting that the lines of a verse begin at
  distinct vertical positions.
- **Switching to the engine's list elements changes list metrics** → indentation
  and spacing will not be identical to today's hand-built blocks, so the
  markup-level list tests are rewritten rather than kept. Expected, and the
  reason the render-level assertions are about *relationships* (same baseline,
  increasing indent) rather than absolute measurements that would be brittle.
- **A description list's presentation changes** → recorded in D2 rather than
  discovered later. It remains within what the requirement asks for.
- **Render-level tests are slower than markup assertions** → they compile a real
  document. `tests/blocks.rs` already does this and the suite runs in about a
  second; the fixtures here are small, and the gate's budget is not at risk.

## Migration Plan

Not applicable in the deployment sense — no released binaries, no downstream
consumers. Within the workspace the order is: build the render helper, then the
paragraph fill (with verse converted in the same step, since one breaks the
other), then the list elements, then heading sizes. Each step leaves the
workspace building and the suite green.

## Open Questions

- Whether a description list should keep the engine's run-in presentation or
  return to a term on its own line. Deferrable: the requirement is satisfied
  either way, the answer wants a real document to judge it against, and changing
  it later touches one emitter function.
