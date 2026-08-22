## Context

See `proposal.md` — *Why*. What matters here is the shape of the boundary
between `asciidoc-parser` and this renderer, because that is where every
decision below is forced.

Today `crates/adocpdf-infra/src/parser.rs` takes paragraph text from
`simple.content().original()` — the **source span** — precisely because the
parser's `rendered()` output is HTML and this renderer does not produce HTML.
That was the right call for a walking skeleton and is the direct cause of the
defects the proposal lists.

Five facts about the upstream crate shape everything that follows. All five
were verified against `asciidoc-parser` 0.29.19 itself, four by reading its
source and one by running it.

1. **There is a supported extension point.**
   `Parser::with_inline_substitution_renderer` is public
   (`src/parser/parser.rs:2286`), and `InlineSubstitutionRenderer`
   (`src/parser/inline_substitution_renderer.rs:44`) is a trait with a default
   body on every method. A downstream renderer overrides what it wants and
   inherits the crate's HTML for the rest.
2. **Inheriting the rest is a trap.** The inherited defaults emit real HTML.
   Run against a probe renderer that overrode only formatting,
   `See https://example.com[the site] and image:x.png[alt] and footnote:[note].`
   rendered as
   `See <a href="https://example.com">the site</a> and <span class="image"><img …></span> and <sup class="footnote">…</sup>.`
   Every one of the trait's seventeen methods must be overridden or HTML tags
   reach the page.
3. **Special characters are substituted first.** `NORMAL_STEPS`
   (`src/content/substitution_group.rs:77`) is `SpecialCharacters`, `Quotes`,
   `AttributeReferences`, `CharacterReplacements`, `Macros`,
   `PostReplacement`, in that order.
4. **Later substitutions are hard-coded against the HTML entities.** The arrow
   replacements are `Regex::new(r"\\?-&gt;")` and siblings
   (`src/content/substitution_step.rs:1148`–`1166`), and the callout matcher
   tests `content.rendered.contains("&lt;")` (`:1329`). A renderer that emits
   anything else for `<` and `>` silently disables them. The probe reproduced
   exactly that: with a non-entity encoding, `a->b` rendered `a-⟨GT⟩b`, no
   arrow.
5. **Passthrough content bypasses the renderer entirely.** `pass:[<raw>]` and
   `+++<raw2>+++` came out of `rendered()` as `<raw> and <raw2>`, verbatim,
   never calling `render_special_character`. Attribute values, by contrast, are
   safe: `:evil: <script>` referenced as `{evil}` came out escaped.

Fact 5 is the security-relevant one and it drives D2.

## Goals / Non-Goals

**Goals:**

- Inline structure obtained from the parser's own extension point, so that
  swapping the parser remains a change to `adocpdf-infra` alone.
- One inline representation used by paragraphs, headings and the document
  title, ending the current disagreement between the two paths.
- An injection boundary that still holds when structure — not just text —
  crosses from source into the emitted markup.

**Non-Goals (design-level; see `proposal.md` for scope non-goals):**

- Reworking `LayoutPlan` into a general document tree. Plan items gain nesting
  where tier-1 constructs need it and no further.
- A rendering-agnostic intermediate representation. `PlanItem` is already the
  seam; introducing a second one buys nothing this tier needs.
- Changing how themes are resolved. Typography gains a field; resolution is
  untouched.

## Decisions

### D1 — Inline structure comes from the parser's renderer hook

Implement `InlineSubstitutionRenderer` in `adocpdf-infra` and register it with
`Parser::with_inline_substitution_renderer`.

*Alternatives considered.* **Parse the parser's HTML output** — needs an HTML
parser, a new dependency, and turns every upstream markup tweak into a
downstream break. **Write an inline parser** — this is the one option that
would genuinely re-open the recorded decision to wrap `asciidoc-parser` rather
than write a parser, and it would do so for the hardest part of the grammar.
**Keep using the source span** — the status quo, and the defect.

The hook is a *port the upstream crate offers*, so using it is the recorded
decision being honoured rather than revisited.

### D2 — Structure crosses the boundary as marked-up text, and the markers are guarded at the door

The trait writes into a `&mut String`, so structure has to survive as
characters in a flat string before being decoded into a tree. Two constraints
collide: fact 4 says special characters **must** be emitted as `&lt;`, `&gt;`
and `&amp;` or the arrow and callout machinery stops working, so the angle
brackets are not available as a marker alphabet; fact 5 says passthrough
content arrives verbatim, so **any** in-band marker is forgeable by an author
who types it.

The decision is therefore in two parts:

- `render_special_character` emits the HTML entities, exactly as upstream does,
  and the decoder converts them back to `<`, `>` and `&` as its last step —
  `&lt;` and `&gt;` before `&amp;`, so that a source `&lt;` (which arrives as
  `&amp;lt;`) round-trips.
- Structural markers are drawn from the Unicode **noncharacters** U+FDD0–U+FDEF
  — the whole reserved block, not a slice of it, so the guard has no gap to
  leak through. Unicode permanently reserves these and forbids them in
  interchange, so no legitimate document contains one. A guard refuses any source containing a
  character in that range, before parsing, in the same place and the same
  manner as the existing `refuse_input_that_would_not_terminate`.

That combination makes the marker alphabet unforgeable by construction rather
than by escaping, which is what lets the injection property be stated as a
property test rather than a list of examples.

*Alternatives considered.* **Private-use characters (U+E000…)** — rejected:
real documents use them for icon fonts, so the guard would refuse valid input.
**A side table of nodes with index markers in the string** — the markers are
still in-band and still forgeable, so it needs the same guard plus more
machinery. **Emitting Typst markup directly from the renderer** — this is the
injection: pass-through text between the renderer's calls would land in the
Typst stream with nothing escaping it.

### D3 — The inline model lives in `adocpdf-core`

`InlineText` becomes a sequence of nodes: literal text, a styled span carrying
one presentation and its own children, and a hard line break. Styles are a
closed set — strong, emphasis, monospace, superscript, subscript, highlight.

Combinations nest rather than being carried as a set on one node: bold-italic
is a styled span inside another. That is the shape the upstream renderer hands
over, since it renders inner content before outer, so a set-valued node would
mean flattening the structure on the way in and inventing it again on the way
out.

Dependency direction is unchanged: this is `adocpdf-core`, which depends on
nothing, and every other layer already depends inward on it. `architecture.toml`
needs no entry, and the guard sees no new edge.

### D4 — Emission stays in code mode, composing content values

The emitter's existing rule — everything in code mode, never markup — is kept.
A styled span becomes a nested call and a sequence becomes a concatenation, so
`Hello *world*.` emits as
`#par(text("Hello ") + strong(text("world")) + text("."))`.

Text still reaches the output only through `markup::string_literal`; structure
is still built only from validated model values. Both rules in `AGENTS.md`
survive intact, which is the point of decoding into a tree before emitting
rather than passing marked-up text through.

**Verified during implementation** (`tests/emission_mechanism.rs`): Typst
content values do concatenate with `+` in code mode, and a `strong` run built
that way resolves to a genuine bold face. The fallback this decision carried —
a content block per item, with its own escaping argument — is not needed.

### D5 — Four more font faces, embedded like the first

`assets/fonts/` gains DejaVuSans-Bold, DejaVuSans-Oblique,
DejaVuSans-BoldOblique and DejaVu Sans Mono (regular and bold), loaded into the
same `FontBook`. Roughly two to three megabytes.

Licence: Bitstream Vera, the same as the existing `DejaVuSans.ttf`, already
recorded in `LICENSING.md` and already satisfied by the bundled
`LICENSE-DejaVu.txt`. No `deny.toml` change: these are assets, not crates, so
no new dependency is added and no new licence obligation is created.

Determinism is unaffected — nothing is discovered from the host, which is the
reason fonts are embedded at all. `Typography` gains a monospace family beside
its body family, and a theme naming a family the book cannot provide is
rejected at construction rather than falling back silently.

### D6 — Plan items nest only where tier-1 needs it

Lists, admonitions and the compound containers hold other blocks, so a flat
`Vec<PlanItem>` cannot express them. `PlanItem` gains a grouping variant
carrying a kind and its children. Theme transitions keep working unchanged
because they are computed during planning, before nesting matters.

*Alternative considered.* Flattening everything with an explicit depth field —
cheaper to emit, but it pushes structural validity into the emitter, where a
malformed depth sequence would produce malformed markup with nothing to catch
it.

### D7 — The inventory is a document with a test behind it

`docs/asciidoc-support.md` carries one row per construct: the construct, its
syntax, whether the parser supports it, whether this renderer honours it, and
its tier — including the rows marked never, for audio, video and docinfo.

A document claiming more than the code does is worse than no document, and the
spec requires the two to agree. So each row marked as honoured carries a
minimal source sample, and a test renders every sample and asserts that nothing
was reported as skipped. The inventory becomes executable rather than
aspirational.

*Trade-off.* This makes the table a test fixture, so a malformed row breaks the
build. That is the intended behaviour, but it means the table's format has to
stay machine-readable, which is a constraint on how freely it can be edited.

### D8 — Inline skips are reported at block granularity

**Verified:** none of the render-params structs carries a span or an offset —
`ImageRenderParams`, `LinkRenderParams`, `XrefRenderParams`,
`FootnoteRenderParams` and the rest expose targets, text and attribute lists
only (`src/parser/inline_substitution_renderer.rs:384`–`585`). The renderer
therefore cannot know where in the source an unsupported inline construct sat.

The skip is therefore carried **in the encoded stream**, not in a side table.
All inline rendering happens in a single pass inside `Parser::parse`, so a
renderer pushing skips into a shared list would produce one flat sequence with
no way to say which block each entry came from. Carried in-band, each skip is
decoded together with the block it sits in, and that block's location is
already in hand.

**A footnote's body cannot be recovered here at all.** Upstream documents
`FootnoteRenderParams::text` as carrying the unresolved ID and being "ignored
in the other cases"
(`src/parser/inline_substitution_renderer.rs:565`–`586`); a resolved footnote's
content lives in the document's catalog, which the HTML converter emits
separately at the foot of the page. Tier 1 therefore renders the marker and
reports the skip, and the specification says so rather than promising text it
cannot reach. Rendering footnote bodies is tier 3 work, and
`docs/asciidoc-support.md` carries the note that makes it a scheduled
commitment rather than a thing that was quietly dropped.

An unsupported inline construct is reported with the location of the block
containing it. That satisfies the specification's "source location" at a
coarser granularity than block-level skips, and the report says which construct
it was, so the author can find it. Anything finer would mean tracking offsets
the upstream API does not provide.

### D9 — Turn on the line breaker the engine was chosen for

**Verified, and the reason this is in scope at all:** the walking skeleton never
enabled Typst's optimal line breaking, so it has never run in this project.
`emitter.rs:110`–`124` emits `font`, `size` and `leading` and nothing else.
`ParElem::justify` defaults to `false`
(`typst-library-0.15.1/src/model/par.rs:241`). And the breaker is selected from
that flag: `linebreaks: base.linebreaks.unwrap_or_else(|| if justify {
Linebreaks::Optimized } else { Linebreaks::Simple })`
(`typst-layout-0.15.1/src/inline/mod.rs:193`). `Linebreaks::Simple` is
documented upstream as "first-fit style … we build lines greedily, always
taking the longest possible line. This may lead to very unbalanced line, but is
fast and simple" (`typst-layout-0.15.1/src/inline/linebreak.rs:163`).

So every page this renderer has produced was laid out by a greedy first-fit
algorithm — the same class Prawn uses, and the opposite of the differentiator
recorded in `openspec/config.yaml`.

The decision is to emit `linebreaks: "optimized"` explicitly and
unconditionally, rather than to reach it by turning justification on. The two
are separable in Typst, and coupling them would force a justified measure on
every document as a side effect of wanting good breaks. Ragged-right text with
optimal breaking is a deliberate typographic choice, not a compromise.

Three settings follow from it, and each becomes theme typography rather than a
constant, because the right answer differs per document:

- **Justification.** Off by default, matching the current appearance; a theme
  may turn it on.
- **Language.** Hyphenation is `auto`, meaning "on if and only if justification
  is enabled" (`typst-library-0.15.1/src/text/mod.rs:545`) *and* "the current
  language is known" (`.../model/par.rs:229`). Nothing emits `lang` today, so
  hyphenation cannot engage even when justification is on. A theme names the
  language.
- **Widow and orphan costs.** Prevention is cost-driven —
  `costs.orphan() > Ratio::zero()`, `costs.widow() > Ratio::zero()`
  (`typst-layout-0.15.1/src/flow/collect.rs:195`–`198`) — and reachable through
  `text(costs: …)`.

*Alternative considered.* Leaving this to a later "typography" change. Rejected:
`Typography` and `emitter::text_style` are already being opened for the
monospace family, the `theming` spec is already being modified, and shipping a
large typography change that leaves the flagship line breaker off would be
difficult to justify afterwards.

Determinism is unaffected: all four are values carried by a theme and emitted
into the markup, not read from the environment.

### D10 — Seven defects found in the parser during this change, and the rule they taught

Task 9.4 ran `cargo-fuzz` against the parse → plan → emit path, as the task
list requires. It found a second non-termination defect in `asciidoc-parser`
0.29.19, unrelated to the inline work: a line of whitespace holding two or more
carriage returns does not return, and libFuzzer minimised it to `"\n\r\r\r"`.

It is refused here rather than in a change of its own, which departs from
`behavioural-testing`'s rule that fixing a fuzz finding is separate work. The
reason is that this is not new behaviour: the `document-rendering` capability
already requires that "the system SHALL return a result for every input it is
given", so the renderer was violating a published requirement of its own. The
delta spec restates that requirement with the new case rather than adding one.

**The condition was measured, not guessed, and the first two attempts were
wrong.** Every string of length four or less over `{CR, LF, tab, space, 'a'}`
was probed against the real parser: 780 inputs, 138 of which do not return. A
first rule — a line holding two or more carriage returns — missed 72 of them
and refused one, `"\r\r\na"`, that renders. What every hang does contain is a
carriage return immediately followed by whitespace that is not a line feed.

**That pattern is necessary but not sufficient**, and the guard refuses on it
anyway. Eighty-one probed inputs contain it and still return. The sufficient
condition depends on block structure — `"= T\n\n\r\r"` hangs while
`"a\n\r\r"` returns — in a way not worth encoding against a defect that should
be fixed upstream rather than modelled here. The specification was amended to
permit a conservative refusal with its cost bounded and stated, which is the
honest description of what this does.

The cost is bounded in the way that matters: CRLF text never contains the
pattern, because every carriage return in it is followed by a line feed. What
is refused is a bare carriage return followed by a space, tab or another
carriage return, which no line-ending convention produces.

**The existing guard turned out to be too narrow in the same way.** Rerunning
the fuzzer after fixing the carriage-return hang produced a third reproducer,
`"[;;\n\u{b}"` — five bytes, with real content in it, hanging on the *vertical
tab* the first guard already knew about. That guard refused only whitespace-only
documents, because every example in hand when it was written had that shape. It
was written against the examples rather than against the defect.

**Three narrower rules were tried for that character, and the fuzzer defeated
each in turn.** First a whitespace-only *document* — defeated by
`"[;;\n\u{b}"`, which has content. Then a line holding nothing else — defeated
by `";toc::  \u{c}"`, where the character shares its line with content. What
survives is a rule about the character itself: a vertical tab or a form feed is
refused anywhere in a document.

That gives up an allowance the earlier change made deliberately —
`"Hello\u{c}world"` used to render — and the specification and its tests now
say so. It is a small price. A vertical tab or form feed in AsciiDoc source is
malformed text in any case: no editor produces one and the language gives it no
meaning.

**A fifth finding, from the property tests rather than the fuzzer.** The
property beside the decoder generates private-use characters among ordinary
content, and it found that `asciidoc-parser` brackets its own cross-reference
placeholders with U+E000 and U+E001, and its footnote markers with U+E002 and
U+E003 (`src/content/content.rs:138`–`154`), on the stated assumption that they
"cannot collide with user text". A document can type them. On `"*\u{e000}<<>>"`
the parser trips its own `debug_assert!` — a panic in a debug build, and a
corrupt placeholder emitted into the rendered text in a release one.

That is precisely the hazard D2 avoids by drawing this crate's markers from
noncharacters, which Unicode forbids in interchange, rather than from the
private use area, which real documents legitimately contain — icon fonts put
glyphs there. It cannot be fixed from here, so the four characters are refused
at the parse boundary alongside this crate's own marker range, and the refusal
is bounded to those four rather than to the whole private use area.

**A sixth finding, and the first that no rule about the source can catch.** The
final fuzz run of task 9.4 returned a *crash* rather than a timeout:
`asciidoc-parser` 0.29.19 panics on an inline `image:` or `icon:` macro with no
target. The regex makes the target group optional and `replace_append` indexes
it unconditionally (`src/content/macros.rs:291`), so `regex` panics with "no
group at index '1'". The fuzzer's ten-byte artifact was `"image:[]\u{2}]"`, but
the trailing bytes are incidental: `image:[alt]` is enough, and so is
`See image:[Figure 1] here.` This is the first of the six that an ordinary
author reaches by accident — forgetting the filename crashes the renderer. Of
the other inline macros, only these two are affected; `link:`, `kbd:`, `btn:`,
`menu:`, `footnote:`, `xref:`, `pass:`, `stem:` and `indexterm:` were each
checked with an empty target and are fine.

**The obvious guard was written, tested, and discarded.** Refusing any source
containing `image:` or `icon:` is outflanked, and demonstrably so rather than
speculatively: attribute references are substituted *before* macros, so

```asciidoc
:foo: imag
{foo}e:[]
```

still panics while containing neither macro name. The dangerous text need not
exist in the source at any point. Shipping the source-text rule would have
meant shipping a rule already known to be incomplete, which is worse than the
narrow rules above — those were at least believed complete when written.

So the unwind is caught at the parse call and reported as the same refusal the
other guards raise. That is sound for every assembly, including ones nobody has
thought of, and it would have caught the fifth finding's `debug_assert!` too.
The cost, stated rather than hidden: this crate's `TypesetRenderer` runs inside
`parse` as a substitution callback, so a panic in *our* code would be reported
as a refusal instead of crashing loudly. The process-wide panic hook is
deliberately left installed — a library must not silence panic reporting for
its host — so the hook's message still reaches stderr ahead of the error, which
is how such a bug would be noticed.

**A seventh, found by the run that verified the sixth was contained.** Twenty
bytes minimising to two consecutive block attribute lines where the first holds
a `%` option shorthand, whitespace and another `%` — `"[% %]\n[f]"` is enough —
trip a `debug_assert!` in `attributes/element_attribute.rs:509` saying that
merging block-style shorthand "should not produce warnings". It does. The guard
from the sixth finding contained it with no change: the parse returns a refusal
and the process lives. That is the first evidence that catching the unwind
generalises, which was the argument for choosing it.

It differs from the sixth in one way worth recording: it is a debug assertion,
so it fires in a debug build and is compiled out of a release one, where the
parser silently discards the warning instead. A document like this is therefore
refused by a debug build and rendered by the shipped one. Nothing here can
close that gap — the defect only exists where the assertion does — so the test
asserts containment in both profiles and the refusal only where the assertion
is compiled in.

**A note on what the fuzzer can still see.** `libfuzzer-sys` installs a panic
hook that calls `abort()` *before* unwinding (`src/lib.rs:91`–`94`), so
`catch_unwind` never runs under the fuzzer. Containing these panics therefore
does not blind the fuzz target to them: it will keep reporting every upstream
panic as a crash, which is the right outcome. The guard is for the shipped
renderer; the fuzzer stays a detector of upstream defects, and each one it
finds gets a test here asserting the guard holds.

**And one defect of this change's own, found by review rather than by either
instrument.** The guard refuses a reserved character in the source, but a
numeric character reference spells it in ASCII — `&#xE000;` — and is resolved by
the decoder *after* the guard has run. So the decoder has to refuse the same two
ranges, and it did refuse one of them: it knew about the noncharacters and not
about the parser's sentinels, which were added to the guard later and to only
one of the two copies of the range. `&#xE000;` walked past a guard written to
refuse U+E000.

The lesson is narrower than the ones above and worth stating separately: a rule
enforced in two places is a rule that will eventually be enforced in one. Both
predicates now live beside the marker alphabet in `crate::inline`, and the guard
imports them, so there is one definition to change.

**The lesson, for whoever reads this next.** Seven defects have now been found
in this one parser — four non-terminating inputs, one in-band marker a document
can forge, and two panics — and every guard written against the *examples in
hand* was later outflanked by an input nobody had tried. Only the guards
written against something structural — a character, or a character followed by
whitespace — have held. Modelling an upstream bug precisely is the wrong goal,
because a bug is not a specification and its shape can change with the next
input. Prefer a rule that cannot be outflanked, state its cost, and delete it
when upstream is fixed. And where the source text cannot decide the question at
all, as with the sixth, do not write a rule about it: contain the failure
instead.

## Risks / Trade-offs

- **Typst content concatenation may not work as D4 assumes** → it is the first
  implementation step, before any of the model work depends on it; the
  fallback is stated in D4.
- **The noncharacter guard refuses a document that previously rendered** → it
  can only do so for a document containing U+FDD0–U+FDEF, which Unicode forbids
  in interchange. The refusal message must say so explicitly, as the existing
  control-character refusal does.
- **An HTML entity inside passthrough content decodes** → `+++&lt;+++` reaches
  the page as `<` rather than as written, because the decoder cannot tell that
  entity apart from one the parser produced. **Resolved during implementation
  by taking this decision's stated fallback:** `Passthrough` exposes `text()`
  and `subs()` but *no offset* into the rendered string, so there are no spans
  to exclude by, and searching for the text would be ambiguous wherever the
  same characters appear elsewhere. The gap is also narrower than it first
  looked — upstream resolves `++…++` and `$$…$$` to `SubstitutionGroup::Verbatim`,
  so special characters are applied there and the round trip is correct; only
  `+++…+++` and `pass:[]` with no substitutions can carry a literal entity
  spelling into the stream. It is recorded in the inventory as a partial-support
  row rather than left silent.
- **Overriding seventeen trait methods is a wide surface** → most are one-line
  "report it and emit the text" implementations, and fact 2 makes leaving one
  un-overridden a visible defect (HTML on the page) rather than a subtle one. A
  test asserting no `<` survives into any rendered text catches the whole class.
- **Two to three megabytes of fonts for formatting most documents use** → the
  alternative is synthetic emboldening, which is visibly worse in a project
  whose stated purpose is to typeset better than the alternatives.
- **Optimal line breaking is global to a paragraph** → changing one word can
  reflow every line of it. That is inherent to Knuth-Plass and is the price of
  the quality; it is recorded here because it bears directly on the deferred
  question of whether partial regeneration is achievable, which
  `render-first-pdf` left open. It makes the answer harder, not different.
- **Catching the parser's panic also catches this crate's own** → the
  `TypesetRenderer` runs inside `parse` as a callback, so a panic in it would
  be reported as a refusal rather than crashing. Accepted because a refusal is
  still a visible failure, the renderer builds strings without indexing or
  unwrapping, and mutation testing is enforced on that file. The panic hook is
  left installed, so such a bug still announces itself on stderr.
- **`InlineText` changing shape touches every layer at once** → unavoidable for
  a model type on the core ring; the compiler finds every site, and the tier-1
  block work lands after the model change rather than beside it.

## Migration Plan

Not applicable in the deployment sense — there are no released binaries and no
downstream consumers. Within the workspace the ordering is: verify D4, change
the core model, thread it through domain and shared, then infra, then the
blocks. `tasks.md` holds the sequence, and each step leaves the workspace
building and the suite green.

## Open Questions

- Whether a monospace face should be a separate family per theme or a single
  workspace-wide face. Deferred safely: the specification requires only that
  monospaced text be set in the theme's monospace family, and either answer
  satisfies it without changing the model's shape.
- What a description list's term should look like typographically — bold on its
  own line, or run-in. A presentation choice the specification deliberately
  leaves open, answerable when the first real document is set.
