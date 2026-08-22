## 1. The model says what an attribute means

- [x] 1.1 Add `InlineStyle::{Underline, Strikethrough, Larger, Smaller}` to
      `adocpdf-core` beside the six styles already there (design D1). Verified
      by the workspace compiling once every match is exhaustive, and by a unit
      test that each new style round-trips through `InlineText`.
- [x] 1.2 Add the block-level presentation the new constructs need — paragraph
      alignment, the lead flag, a list's marker shape, its declared start and
      its form (default, horizontal, Q&A, checklist) — as value objects that
      validate on construction. Verified by unit tests that an invalid value is
      not representable, following the crate's existing value objects.

## 2. The mapper reads the attribute lists

- [x] 2.1 Read the inline attribute list in `TypesetRenderer`, mapping the roles
      design D3 names to the new styles and encoding them with the marker
      alphabet. Verified by a decoder test that `[.underline]#x#` yields an
      `Underline` node, and by the injection property still holding.
- [x] 2.2 Report a role that is not in the honoured set through
      `DecodedInline::unsupported`, keeping its text (design D2). Verified by a
      test that `[.myrole]#text#` renders `text`, reports `role "myrole"`, and
      leaves the run set no differently from body text.
- [x] 2.3 Read a paragraph's attribute list for alignment and the lead flag.
      Verified by a mapper test over each of the five attributes.
- [x] 2.4 Read a list's attribute list for marker shape, declared start and
      form, and read checklist markers from the item text. Verified by a mapper
      test per variant, including that the `[x]` syntax does not survive into
      the item's text.
- [x] 2.5 Map `[discrete]` to a heading block rather than a section (design D4).
      Verified by a mapper test that a discrete heading leaves the level of a
      following section heading unchanged, and that the blocks after it belong
      to the enclosing section.

## 3. Emission

- [x] 3.1 Emit the four new inline styles. Verified by a render-level test that
      an underlined run carries a rule and a struck run carries one through it,
      and that a larger run is set larger than body text.
- [x] 3.2 Emit paragraph alignment and the lead paragraph, per paragraph rather
      than by changing the theme (design D5). Verified by a render-level test
      that a centred paragraph's lines are centred while its neighbours are not,
      and that a justified paragraph is flush under an unjustified theme.
- [x] 3.3 Emit a list's declared marker shape and start number through the
      engine's own list elements. Verified by a render-level test that a list
      declaring a start counts from it, and one declaring a marker shows it.
- [x] 3.4 Emit horizontal and Q&A description lists. Verified by a render-level
      test that a horizontal list sets each term beside its description with the
      terms aligned, rather than in front of it.
- [x] 3.5 Emit checklist markers showing state, with no bracket syntax reaching
      the page. Verified by a render-level test over a checked and an unchecked
      item.
- [x] 3.6 Emit a discrete heading with heading typography and no section
      structure. Verified by a render-level test that it is set larger than body
      text and that the heading after it keeps its own level's size.

## 4. Inventory, documentation and the gate

- [x] 4.1 Move every row this change honours from `scheduled` to `honoured` in
      `docs/asciidoc-support.md`, with a sample for each, and record the roles
      that are deliberately not honoured. Verified by a green
      `tests/support_inventory.rs`, which renders every sample and checks the
      claim.
- [x] 4.2 Update `README.md`'s "What works today" for roles, list variants and
      discrete headings. Verified by review.
- [x] 4.3 Extend the tier-1 showcase document with the new constructs, render
      it, and read the pages rather than trusting the suite. Verified by
      underline and strikethrough visible, a checklist showing state, a
      horizontal list aligned, and a discrete heading not disturbing the
      sections around it.
- [x] 4.4 Run `scripts/ci/gate.sh` and hold the 90% coverage floor without
      lowering it. Verified by a green gate across all seventeen jobs.
