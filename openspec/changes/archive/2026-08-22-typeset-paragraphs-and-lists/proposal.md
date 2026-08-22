## Why

The first document rendering every tier-1 construct was produced after
`render-inline-and-core-blocks` was complete, and looking at its six pages found
three defects that 412 passing tests could not see. The tests assert that the
emitted markup *contains* the right pieces; every one of those assertions is
true of markup that lays out wrongly.

One of the three matters more than the other two. Paragraph text is handed to
the layout engine inside a string literal, and a newline inside a string is a
line break rather than a space — so a source file wrapped at 76 columns, which
is how almost everyone writes AsciiDoc, arrives on the page pre-broken at 76
columns. The optimal line breaker that `render-inline-and-core-blocks` was
extended to enable has nothing left to optimise, and the `theming` requirement
that a paragraph "SHALL be broken by optimising the paragraph as a whole" is
met in the markup and defeated in the output.

## What Changes

- **Paragraph text is filled, not pre-broken.** A newline that AsciiDoc treats
  as a soft line break — an ordinary wrap inside a paragraph — becomes a space
  before it reaches the layout engine. An explicit hard break stays a break.
  This is what makes optimal line breaking observable, and it is also why the
  landscape `wide` theme currently leaves half its page empty: the paragraph was
  already broken at the portrait measure.
- **A list marker sits on the same line as its item's text.** Every item
  currently emits its marker and then the item's text as a *block*, which
  begins a new line, so bullets sit alone above their words. Ordered items look
  correct only because the emitted `1. ` is re-parsed by the engine as its own
  enumeration syntax — emitter-generated text being read as markup, which is
  the discipline the injection boundary exists to keep. Both cases want the
  marker and the first line of the item in one paragraph.
- **Heading levels 3, 4 and 5 are told apart.** They render, but identically,
  so a reader cannot recover the document's structure from the page. The
  existing requirement already asks for levels to be "visually distinguished
  from each other"; this makes it true below level 2.
- **A render-level test instrument.** All three defects survived because
  nothing in the suite lays out a page and measures it. Fixtures are rendered
  and their *page geometry* compared — where a line broke, what column an item's
  text starts in, how tall a heading is set — rather than the markup that
  produced them. This is the change's most durable part: without it the next
  layout defect is equally invisible.

## Capabilities

### New Capabilities

None. Every behaviour here is already required by an existing capability and is
not being delivered; the deltas below sharpen the requirements so that a
renderer which lays out wrongly can no longer satisfy them.

### Modified Capabilities

- `inline-formatting`: "Hard line breaks are honoured" already says a line
  ending without a break marker "SHALL continue to be reflowed", so the
  requirement is right and the renderer does not meet it. What it lacks is a
  scenario that can catch the failure: it gains three, tying the break
  positions to the measure rather than to the source, and requiring the two
  kinds of line ending to be distinguishable within one paragraph.
- `block-constructs`: "Lists render with their structure" gains a scenario
  requiring a marker to sit beside the text it marks, and requiring an ordered
  item's number to be the one the renderer determined rather than one the
  layout engine inferred from the text.
- `document-rendering`: "Supported document constructs" gains a scenario
  requiring heading levels below the second to be distinguishable from one
  another.

## Non-goals

- **Sibling and nested lists.** The proof sheet appeared to show three
  top-level lists collapsing into one nested structure with a description
  list's terms lost. Checking the parse tree showed this is not a defect here:
  in AsciiDoc a blank line does not end a list, and a list with a different
  marker nests inside the item above it, so the tree the parser returns is
  correct and the renderer honours it. One genuine oddity remains — at that
  nesting depth `asciidoc-parser` folds a description item into the enclosing
  ordered list and loses its term — but it is upstream, and it is not this
  change's to fix.
- **Justification and hyphenation defaults.** The theme can already ask for
  both; which the built-in themes should ask for is a separate question about
  the default theme's design.
- **Table, image, footnote and cross-reference rendering.** Scheduled in
  `docs/asciidoc-support.md` for later tiers, and untouched here.
- **Incremental rendering.** Still open, and filling paragraphs properly makes
  it neither easier nor harder than optimal breaking already did.

## Impact

**Layers.** `adocpdf-infra` only — `emitter.rs` for the marker and heading
work, and either `inline.rs`'s decoder or the emitter for the soft-break
collapse. No change to `adocpdf-core`'s model, to `adocpdf-domain`, or to any
delivery crate. `InlineNode::LineBreak` already distinguishes the hard break, so
the model has the information needed and only the emission is wrong.

**Tests.** New render-level fixtures alongside the existing `blocks.rs`
`rendered` module, which already compiles documents and reads laid-out frames —
the instrument is an extension of what is there, not a new dependency. The
existing emitted-markup tests stay: they are cheap and they localise a failure.

**Documentation.** `README.md` claims optimal line breaking as a headline
feature; it becomes true rather than nominal. `docs/asciidoc-support.md` rows
for hard line breaks and for lists are checked by a test that renders each
sample, so they gain teeth without changing wording.

**Risk.** Collapsing soft newlines touches every paragraph in every document —
the widest-reaching edit in the change — which is precisely why the render-level
instrument is built first rather than last.
