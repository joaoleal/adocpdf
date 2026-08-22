# inline-formatting Specification

## Purpose

Recognising the structure inside a run of AsciiDoc text — emphasis, strength,
monospace, typographic substitutions and the rest — and setting it on the page,
including what becomes of an inline construct the renderer cannot yet honour
and the guarantee that no inline content can turn into a rendering instruction.

## Requirements

### Requirement: Inline formatting is honoured

The system SHALL recognise the inline formatting the AsciiDoc language defines
for bold, italic, monospace, superscript, subscript and highlighted text, in
both their constrained and unconstrained forms, and SHALL set each in a manner
visually distinct from surrounding body text.

The delimiters themselves SHALL NOT appear in the output. Formatting SHALL
nest: a construct inside another SHALL carry both presentations.

This SHALL apply wherever inline text appears — in a paragraph, in a section
heading and in the document title alike.

#### Scenario: A bold word is set in bold

- **WHEN** a paragraph contains `A bold *word* here`
- **THEN** the rendered text reads `A bold word here`
- **AND** the asterisks do not appear in the output
- **AND** `word` is set in a heavier face than the text around it

#### Scenario: Formatting nests

- **WHEN** a paragraph contains text marked both bold and italic, one inside
  the other
- **THEN** the affected text carries both presentations

#### Scenario: Formatting inside a heading is honoured

- **WHEN** a section heading or the document title contains inline formatting
- **THEN** the heading renders with that formatting applied
- **AND** no markup of any kind appears in the heading text

#### Scenario: An unmatched delimiter stays literal

- **WHEN** a paragraph contains a formatting delimiter with no closing partner
- **THEN** the delimiter renders as an ordinary character
- **AND** the rest of the paragraph is unaffected
### Requirement: Typographic substitutions are applied

The system SHALL apply the character replacements the AsciiDoc language
defines: the copyright, registered and trademark signs; em dashes; ellipses;
single and double arrows; the typographic apostrophe; and curved quotation
marks.

An author SHALL be able to suppress any of them by escaping.

#### Scenario: Replacements reach the page as their typographic form

- **WHEN** a paragraph contains `(C) 2026 -- see p. 3 ... a -> b`
- **THEN** the rendered text contains `©`, an em dash, an ellipsis and `→`
- **AND** none of the source spellings remain

#### Scenario: Curved quotation marks are applied

- **WHEN** a paragraph quotes text using the curved-quote syntax
- **THEN** the rendered text carries opening and closing curved quotation marks

#### Scenario: An escaped replacement stays literal

- **WHEN** a paragraph contains a replacement sequence preceded by a backslash
- **THEN** the sequence renders exactly as written, without the backslash
- **AND** no substitution occurs
### Requirement: Attribute references resolve

An attribute reference in inline text SHALL be replaced by the attribute's
value, whether the attribute is built in or defined by the document.

A reference to an attribute that is not set SHALL NOT fail the render. The
reference SHALL be left as the author wrote it and SHALL be reported, so that a
missing definition is visible rather than silently producing empty text.

#### Scenario: A defined attribute is substituted

- **WHEN** a document defines an attribute and a later paragraph references it
- **THEN** the attribute's value appears in the rendered text
- **AND** the reference syntax does not

#### Scenario: An undefined attribute is reported, not dropped

- **WHEN** a paragraph references an attribute that is never defined
- **THEN** the render succeeds
- **AND** the reference appears in the output as written
- **AND** the report names the unresolved attribute and its source location
### Requirement: Hard line breaks are honoured

The system SHALL break a line where the author has asked for a break, both
through a trailing break marker on an individual line and through the
block-level option that makes every line in a block break.

A line ending without a break marker SHALL continue to be reflowed by the
layout engine. Where a paragraph's lines break SHALL be determined by the
measure the theme gives it, not by where the source file happened to wrap: two
documents whose paragraphs differ only in their source line endings SHALL
produce the same pages.

A paragraph SHALL be able to contain both kinds of line ending, and the two
SHALL be distinguishable on the page.

#### Scenario: A trailing break marker breaks the line

- **WHEN** a paragraph line ends with a hard line break marker
- **THEN** the following text starts on a new line
- **AND** the marker does not appear in the output

#### Scenario: An ordinary line wrap is unaffected

- **WHEN** a paragraph spans several source lines with no break markers
- **THEN** the text is set as one reflowed paragraph
- **AND** its lines end where the measure requires, not where the source wrapped

#### Scenario: Source wrapping does not reach the page

- **WHEN** the same paragraph is written once hard-wrapped over several source
  lines and once as a single long source line
- **THEN** both render identically

#### Scenario: A wider measure produces longer lines

- **WHEN** a paragraph written over several source lines is rendered under a
  theme whose page is wider
- **THEN** its lines grow to use the wider measure

#### Scenario: Both kinds of line ending in one paragraph

- **WHEN** a paragraph contains a hard line break and also wraps in the source
- **THEN** the text breaks at the hard break
- **AND** it does not break where the source merely wrapped
### Requirement: Escaped and passthrough content is literal

Text the author has escaped, and text inside an inline passthrough, SHALL reach
the page exactly as written, with no substitution applied and with the escape
or passthrough syntax itself removed.

Because the output is a typeset page rather than a markup document, passthrough
content SHALL NOT be interpreted as markup of any kind. It is literal text.

#### Scenario: An escaped delimiter is not formatting

- **WHEN** a paragraph contains a formatting delimiter preceded by a backslash
- **THEN** the delimiter appears in the output as an ordinary character
- **AND** the text around it is not formatted

#### Scenario: Passthrough content is set verbatim

- **WHEN** a paragraph contains an inline passthrough holding text that would
  otherwise be substituted
- **THEN** that text appears exactly as written
- **AND** no substitution has been applied to it
### Requirement: Inline content cannot alter rendering instructions

No inline content SHALL be able to introduce, terminate or alter a rendering
instruction, whatever channel it arrives through.

This SHALL hold for content that reaches the renderer without substitution —
passthrough content in particular — as well as for ordinary text. Content that
resembles the renderer's own internal encoding of inline structure SHALL be
treated as text, not as structure.

#### Scenario: Passthrough content cannot forge inline structure

- **WHEN** an inline passthrough contains text matching whatever encoding the
  renderer uses internally to represent inline structure
- **THEN** that text renders literally
- **AND** no formatting, page or typography change results from it

#### Scenario: Layout-control syntax inside formatting stays literal

- **WHEN** formatted inline text contains characters meaningful to the
  rendering layer's own markup syntax
- **THEN** those characters appear verbatim in the output
- **AND** the formatting is applied around them as it would be around ordinary
  letters
### Requirement: An unsupported inline construct is reported, and its text kept

Encountering an inline construct outside the supported set SHALL NOT abort the
render and SHALL NOT discard the text the construct carried.

The system SHALL render that text as ordinary inline text and SHALL report the
construct by name with its source location, in the same manner as an
unsupported block.

Where a construct's text is not carried inline — a footnote's body, for
instance, belongs to the document rather than to the point that references it —
the system SHALL render the marker the construct occupies in the text and SHALL
report it as skipped. The text SHALL NOT be silently discarded: it is
unreachable at this point, and the published support inventory SHALL record
which constructs this applies to and where they are scheduled to be honoured in
full.

#### Scenario: An inline macro renders its text and is reported

- **WHEN** a paragraph contains a link, a footnote, a cross-reference or an
  inline image
- **THEN** the render succeeds
- **AND** the text the construct carried appears on the page
- **AND** the report names the construct and its source location

#### Scenario: A construct whose text is not carried inline keeps its marker

- **WHEN** a paragraph contains a footnote
- **THEN** the render succeeds
- **AND** the point in the text where the footnote sits remains visible as a
  marker
- **AND** the report names the footnote and its source location
- **AND** the support inventory records that its body is not yet rendered, and
  when it will be

#### Scenario: No markup from an unsupported construct reaches the page

- **WHEN** a paragraph contains any inline macro the renderer does not support
- **THEN** no markup, tag or attribute from that construct appears in the
  rendered text

### Requirement: Inline roles with typographic meaning are honoured

The system SHALL honour the inline roles the AsciiDoc language defines with a
typographic meaning — underline, strikethrough, and the relative sizes larger
and smaller — applying them to the text they enclose and composing them with
the styles already honoured.

A role the system cannot honour SHALL be reported as a skipped construct, named,
and the text it enclosed SHALL still be rendered. A role is a stylesheet class
by origin, and this renderer has no stylesheet: inventing a presentation for an
unknown role would put on the page something the author did not ask for.

#### Scenario: An underlined span is underlined

- **WHEN** a span carries the underline role
- **THEN** the text is underlined on the page
- **AND** nothing is reported as skipped

#### Scenario: A struck-through span is struck through

- **WHEN** a span carries the strikethrough role
- **THEN** the text is struck through on the page

#### Scenario: A relative size role changes the size

- **WHEN** a span carries the larger role and another carries the smaller role
- **THEN** the first is set larger than body text and the second smaller

#### Scenario: A role that cannot be honoured is reported, not guessed at

- **WHEN** a span carries a role this renderer has no meaning for
- **THEN** the text it enclosed still appears on the page
- **AND** the role is named in the skipped report
- **AND** the text is set no differently from the body text around it

#### Scenario: A role composes with a style

- **WHEN** a span is both underlined by a role and bold
- **THEN** both are visible on the page
