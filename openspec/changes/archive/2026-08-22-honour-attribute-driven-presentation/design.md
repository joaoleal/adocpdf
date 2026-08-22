## Context

See `proposal.md` — Why. Three facts, verified against the installed
dependencies rather than assumed:

1. **Inline roles already reach this renderer and are discarded.**
   `InlineSubstitutionRenderer::render_quoted_substitution` is handed
   `attrlist: Option<Attrlist<'_>>` and `id: Option<String>`
   (`asciidoc-parser-0.29.19/src/parser/inline_substitution_renderer.rs:57`–`65`).
   `TypesetRenderer` takes only the `QuoteType` and ignores the rest, so the
   information needed for `[.underline]#x#` is on the doorstep today.
2. **Block attribute lists are already read.** The mapper reads `theme=` from a
   section's attrlist, so the mechanism exists and this change widens what it
   looks for rather than introducing it.
3. **The engine has the presentation primitives.** `list` takes a `marker`,
   `enum` takes a `start`, `terms` a `separator` and `tight`, and `par` a
   `justify`; `align` and `underline`/`strike` are elements in their own right.
   Nothing here needs a primitive that does not exist.

## Goals / Non-Goals

**Goals:**

- One place decides what an attribute list means, so a role does not mean one
  thing on a paragraph and another on a list.
- An attribute this renderer cannot honour costs the reader nothing: the
  content renders, and the attribute is reported by name.
- The honoured vocabulary is written down in the specs, not discovered by
  reading the implementation.

**Non-Goals:**

- Author-defined roles. That is a theme-file feature and the theme file does not
  exist; D3 records the shape it should take when it does.
- Carrying ids anywhere. An id is only useful once something can link to it, and
  cross-references are tier 3. Reading ids without using them would be dead
  weight in the model.

## Decisions

### D1 — A closed vocabulary of roles, in the core model, not strings in infra

Roles that this renderer honours become model values —
`InlineStyle::{Underline, Strikethrough, Larger, Smaller}` beside the six styles
already there, and a `Presentation` on paragraphs and lists carrying alignment,
lead, marker shape, start number and list form.

The alternative is to carry role *strings* from the parser to the emitter and
match on them where they are used. Rejected for the reason the injection
boundary exists: a string that reaches the emitter is a string that has to be
proven safe there, and every new use is a new place to get it wrong. A value
object is proven safe once, at construction, and the compiler finds every site
that must handle a new variant.

**Dependency direction.** `adocpdf-core` gains the variants and `adocpdf-infra`
maps to them; nothing flows outward, and `architecture.toml` is unchanged.

### D2 — An unhonoured role is reported through the path that already exists

`DecodedInline::unsupported` already carries the constructs a run of inline
content could not honour, and the mapper already turns those into
`SkippedConstruct`s with the enclosing block's location. An unknown role joins
that list as `role "warning"`.

This is deliberately the same channel as an unsupported macro, because it is the
same promise: the omission is never silent and the text survives. Inventing a
second reporting path would give an author two things to check.

### D3 — The honoured set is exactly the roles the language documents

`underline`, `line-through`, `big`, `small`, `text-left`, `text-center`,
`text-right`, `text-justify`, `lead`. Nothing else.

The temptation is to pass unknown roles through to some styling hook so that
"it works if you configure it". There is nothing to configure: no theme file
exists, so a document cannot say what `[.warning]` should look like. When themes
become authorable the natural design is a theme naming the roles it styles, at
which point this list stops being closed and becomes the theme's default. That
is a later change and the specs are written so it does not contradict them.

**Unverified, and marked as such:** whether Asciidoctor treats `big` and `small`
as exactly one step in each direction is not something this design checked. The
requirement asks only that larger is larger and smaller is smaller than body
text, which is the part a reader can see.

### D4 — A discrete heading is a block, not a section

`[discrete]` is honoured in the mapper by emitting a heading block rather than
opening a `Section`. The document model already distinguishes the two — a block
title is set apart from a heading and takes no part in nesting — and this is the
same shape with heading typography instead of title typography.

The alternative, a flag on `Section` that the planner learns to ignore, spreads
the concept across two layers to save one variant. A discrete heading genuinely
is not a section; the model should say so.

### D5 — Paragraph alignment overrides the theme for one paragraph

Alignment is emitted per paragraph rather than by changing the theme's
justification, so it cannot leak into the paragraphs around it. The `theming`
requirement that layout settings come from the theme and not the environment is
untouched: this is the *document* asking, which is exactly what an attribute
list is for, and it is as reproducible as the source that carries it.

**Determinism boundary.** Nothing here reads the environment. The same source
under the same theme still produces the same bytes.

## Risks / Trade-offs

- **The role vocabulary is open-ended and this change closes it** → an author
  using `[.myrole]` gets a report rather than a rendering. Deliberate, stated in
  D3, and the alternative is inventing presentation nobody asked for.
- **`InlineStyle` gains four variants and every match on it must handle them** →
  the compiler finds them all, which is the argument for D1 over strings.
- **Horizontal description lists interact with the measure** → a long term in a
  horizontal list squeezes the description column. The requirement asks only
  that terms align and descriptions sit beside them; the column policy is an
  implementation choice that can change without touching the spec.
- **Checklists are unordered lists with a marker that is content** → the marker
  must show state without the `[x]` syntax reaching the page, which is the same
  "content is never markup" rule the emitter already follows for list markers.

## Migration Plan

Not applicable — no released binaries, no downstream consumers. Order: the core
model first (it is what everything else names), then the mapper reading the
attribute lists, then emission, then the inventory rows. Each step leaves the
workspace building and the suite green.

## Open Questions

- Whether a lead paragraph should be larger, or set in a different weight, or
  both. Deferrable: the requirement asks that it be distinct, any of these
  satisfies it, and the answer wants a real document to judge — the same reason
  the description-list question was deferrable, and that one is now answered in
  the proposal.
