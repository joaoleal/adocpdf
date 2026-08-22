## Why

Tier 1 rendered the constructs AsciiDoc spells with punctuation. Tier 2 is
mostly one mechanism: the **attribute list** — `[.role]`, `[circle]`,
`[start=4]`, `[horizontal]`, `[discrete]` — which the parser already hands over
in full and which this renderer ignores everywhere except `[theme=…]`.

That gap is visible to an author in an ordinary way. `[.underline]#term#`
renders as plain text, `[start=4]` renumbers from one, `* [x] done` puts a
literal `[x]` on the page, and `[discrete]` promotes a heading into the section
hierarchy where the author asked for the opposite. None of these are exotic;
they are the second thing people write after they learn the first.

## What Changes

- **Inline roles that mean something typographically are honoured**:
  `[.underline]`, `[.line-through]`, `[.big]` and `[.small]`. A role reaches
  this renderer already — `render_quoted_substitution` is handed the attribute
  list and the id — and is discarded today.
- **A role with no typographic meaning is reported, and its text still
  rendered.** A role is a CSS class by origin: `[.warning]#x#` means whatever a
  stylesheet says it means, and there is no stylesheet here. Reporting it by
  name is honest; inventing a presentation for it is not.
- **Paragraph alignment and the lead paragraph**: `[.text-left]`,
  `[.text-center]`, `[.text-right]`, `[.text-justify]` and `[.lead]`.
- **List variants**: a custom marker (`[circle]`, `[square]`, `[disc]`), an
  explicit start (`[start=4]`), horizontal and Q&A description lists
  (`[horizontal]`, `[qanda]`), and checklists (`* [x]`, `* [ ]`).
- **Discrete headings**: `[discrete]` sets a heading that takes no part in the
  section hierarchy — which also means it must not affect the nesting of the
  sections around it.

## Capabilities

### New Capabilities

None. Every construct here belongs to a capability that already exists.

### Modified Capabilities

- `inline-formatting`: a new requirement for role-driven inline styles, and for
  reporting a role this renderer cannot honour while keeping its text.
- `block-constructs`: a new requirement for the list variants, and one for
  paragraph presentation attributes. "Lists render with their structure" is
  modified so that an ordered list's numbering is the *declared* start rather
  than always one.
- `document-rendering`: "Supported document constructs" is modified for the
  discrete heading, which is a heading that is deliberately not a section.

## Non-goals

- **The rest of tier 2**, which is document-level machinery rather than
  attribute handling and wants a change of its own: auto and custom IDs,
  section numbering, parts and chapters (`:doctype: book`), special sections,
  author and revision lines, subtitles, and the table of contents. Several of
  those only pay off once cross-references exist in tier 3, and an ID that
  nothing can link to is an anchor with no anchorage.
- **Arbitrary role styling.** There is no theme file yet, so a document cannot
  say what `[.warning]` should look like. When themes become authorable, the
  natural design is for a theme to name roles it styles; until then an unknown
  role is reported, not guessed at.
- **`[%collapsible]`**, which the inventory schedules for tier 4: a PDF page
  cannot collapse.

## Impact

**Layers.** `adocpdf-core` gains inline styles (underline, strikethrough, size)
and the block-level presentation the new list variants need, so the model
changes and the change is not confined to one crate. `adocpdf-infra` reads the
attribute lists in the mapper and emits the new presentation. `adocpdf-domain`
is untouched: none of this is a planning decision.

**The description-list question is answered by this change, not left open.**
`typeset-paragraphs-and-lists` deferred whether a description list should keep
the engine's run-in term or return to a term on its own line. Both were
rendered and compared: run-in keeps the term and its description reading as one
unit with the continuation hanging under it, while the stacked form detaches
them by a paragraph's space and reads as twice as many unrelated paragraphs.
Run-in stays, and `[horizontal]` and `[qanda]` are the variants an author uses
when they want something else — which is the reason the question could be
deferred safely.

**Risk.** The role vocabulary is open-ended by design, so the honest boundary is
"honour the ones the language documents, report the rest". Getting that boundary
wrong in the generous direction means inventing presentation the author did not
ask for; getting it wrong in the strict direction means noise in the skipped
report. The delta specs pin the honoured set rather than leaving it to the
implementation.
