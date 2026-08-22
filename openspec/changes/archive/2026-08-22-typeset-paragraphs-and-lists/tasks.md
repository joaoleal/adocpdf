## 1. The instrument

Built first, and deliberately so: every defect in this change survived because
nothing could ask a question about a laid-out page. See design D4.

- [x] 1.1 Add a test-only helper that renders a source document through the real
      parse → plan → emit → compile path and returns, for each page, every text
      run with its position, font family and size. Generalises what
      `tests/blocks.rs`'s `rendered` module already does from "the text on a
      page" to "where each run sits and how it is set". Verified by a test that
      renders a one-line document and asserts the run's family, size and that
      its position lies inside the page's margins.
- [x] 1.2 Add helpers over that raw output for the three questions this change
      asks: the lines of a paragraph (runs grouped by baseline, in reading
      order), whether two runs share a baseline, and the leftmost x of a line.
      Verified by a test over a two-line paragraph asserting it reports exactly
      two lines, in order.
- [x] 1.3 Share the helper with `tests/blocks.rs` rather than leaving a second
      copy there, and confirm the existing `rendered` tests still pass against
      it unchanged. Verified by a green `cargo test -p adocpdf-infra`.

## 2. Paragraphs are filled, not pre-broken

- [x] 2.1 Write the failing test first as a pair, using the helper from 1.2: the
      same paragraph written hard-wrapped over several source lines and as one
      long source line must render to the same lines. Verified by watching it
      fail for the right reason — the wrapped version reports one line per
      source line — before any fix is made.
- [x] 2.2 Collapse a newline inside `InlineNode::Text`, with the horizontal
      whitespace either side of it, to a single space as the inline tree is
      built, leaving `InlineNode::LineBreak` untouched (design D1). Verified by
      2.1's test passing and by a unit test that a text node built from a
      wrapped source carries no newline.
- [x] 2.3 Convert a verse block's line endings into `InlineNode::LineBreak` in
      the mapper, so a verse keeps its breaks for the reason it means them
      rather than by string-encoding accident. Verified by a render-level test
      that a three-line verse occupies three distinct baselines — the existing
      markup-level assertion cannot see this and is replaced by it.
- [x] 2.4 Add the render-level test that a paragraph's lines end where the
      measure requires: the same wrapped paragraph under the `wide` theme uses
      longer lines than under the default. Verified by the assertion that the
      landscape rendering has strictly fewer lines.
- [x] 2.5 Add the render-level test that a paragraph holding both a hard break
      and an ordinary source wrap breaks at the first and not the second.
      Verified by the reported line count and by the text of each line.

## 3. Lists use the engine's own elements

- [x] 3.1 Write the failing test first: a short unordered item's marker and the
      first word of its text share a baseline, and no line of output holds the
      marker alone. Verified by watching it fail before the fix.
- [x] 3.2 Emit unordered items as the engine's list items with the item's
      content passed as an argument rather than written after a literal marker
      (design D2). Verified by 3.1 passing.
- [x] 3.3 Emit ordered items as the engine's enum items, passing the position
      the renderer determined as the explicit number. Verified by a test that
      the numbers on the page are the renderer's — including a list whose items
      the renderer numbers from something other than one — and by no `1. `
      appearing as literal text in the emitted markup.
- [x] 3.4 Emit description items as the engine's term items, each term with its
      description. Verified by a render-level test that both a term and its
      description appear and that the term is set apart from it; the
      presentation change this brings is recorded in design D2.
- [x] 3.5 Add the render-level test for hanging indent: an item long enough to
      wrap sets its second line under the item's text, not under the marker.
      This is the latent defect the hand-built blocks carried and no sample in
      the proof sheet exposed. Verified by comparing the leftmost x of the two
      lines.
- [x] 3.6 Add the render-level test that a nested list is indented relative to
      its parent, replacing the markup-level assertion. Verified by comparing
      the leftmost x of an item at each level.
- [x] 3.7 Rewrite the markup-level list tests that assert on the old
      hand-built block shape, keeping any whose assertion still holds. Verified
      by a green suite and by no test asserting on a marker written as text.

## 4. Heading levels are told apart

- [x] 4.1 Emit a text size per heading level, derived from the theme's body
      size, for every level the renderer honours (design D3). Verified by a
      render-level test that a document with headings at every level sets no two
      levels identically and every level apart from body text.

## 5. Documentation and the gate

- [x] 5.1 Update `docs/asciidoc-support.md` where a row's claim changes meaning
      — the hard line break row and the list rows — and confirm the inventory
      test still renders every sample. Verified by a green
      `tests/support_inventory.rs`.
- [x] 5.2 Re-render the tier-1 showcase document used to find these defects and
      read the pages, rather than trusting the suite. Verified by paragraphs
      filling the measure, markers sitting beside their text, and the wide
      section using its full width.
- [x] 5.3 Run `scripts/ci/gate.sh` and hold the 90% coverage floor without
      lowering it. Verified by a green gate across all seventeen jobs.
