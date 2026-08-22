# block-constructs Specification

## Purpose

The block-level constructs this renderer honours beyond the section and the
paragraph — verbatim blocks, admonitions, quotations, the compound containers,
lists, breaks and block titles — and how each is presented on a typeset page.

## Requirements

### Requirement: Verbatim blocks preserve their content exactly

The system SHALL render literal, listing and source blocks with their content
preserved exactly as written: every space, line break and delimiter character
intact, no substitution applied, and no reflowing by the layout engine.

Verbatim content SHALL be set in a monospaced face, visually distinguished from
body text.

A source block SHALL render its content verbatim whether or not a language is
declared. The declared language SHALL NOT change the presentation.

#### Scenario: A listing block keeps its whitespace

- **WHEN** a listing block contains indented lines and blank lines
- **THEN** the output preserves the indentation and the blank lines
- **AND** the lines are not reflowed to fill the measure

#### Scenario: Verbatim content is not substituted

- **WHEN** a verbatim block contains text that would otherwise be formatting,
  an attribute reference or a character replacement
- **THEN** that text appears exactly as written
- **AND** no substitution has been applied

#### Scenario: An unterminated verbatim block does not abort the render

- **WHEN** a verbatim block's opening delimiter has no matching close
- **THEN** the render succeeds
- **AND** what the source could be understood to mean is rendered
- **AND** anything that could not be represented is reported
### Requirement: Admonitions are labelled and set apart

The system SHALL render the five admonition kinds the AsciiDoc language defines
— note, tip, important, caution and warning — in both the single-paragraph form
and the delimited-block form.

An admonition SHALL be visually set apart from the body text around it and
SHALL carry a label identifying which kind it is.

#### Scenario: A paragraph-form admonition is labelled

- **WHEN** a paragraph begins with an admonition marker
- **THEN** the paragraph renders set apart from surrounding body text
- **AND** the output carries a label naming the admonition kind

#### Scenario: A block-form admonition may hold several blocks

- **WHEN** a delimited admonition contains more than one paragraph
- **THEN** all of its content renders inside the same set-apart region
- **AND** the region carries one label
### Requirement: Quotations carry their attribution

The system SHALL render quote blocks and verse blocks, and SHALL render the
attribution and citation title where the source supplies them.

A verse SHALL preserve the line breaks of its content; a quote SHALL allow its
content to be reflowed.

#### Scenario: A quote renders with its attribution

- **WHEN** a quote block declares an attribution and a citation title
- **THEN** the quoted text renders set apart from body text
- **AND** the attribution and citation appear with it

#### Scenario: A verse keeps its line breaks

- **WHEN** a verse block contains several short lines
- **THEN** each line begins on a new line in the output

#### Scenario: A quote without attribution still renders

- **WHEN** a quote block declares neither attribution nor citation
- **THEN** the quoted text renders
- **AND** no empty attribution line appears
### Requirement: Compound blocks contain other blocks

The system SHALL render example blocks, sidebars and open blocks, and SHALL
render the blocks nested inside them.

An example block and a sidebar SHALL each be visually distinguishable from body
text and from one another. An open block SHALL group its content without
imposing a presentation of its own.

Nesting SHALL be honoured to the depth the source uses.

#### Scenario: A sidebar renders its nested content

- **WHEN** a sidebar contains a paragraph and a list
- **THEN** both render inside the sidebar's region

#### Scenario: Compound blocks nest

- **WHEN** an example block contains a sidebar containing a paragraph
- **THEN** the nesting is visible in the output
### Requirement: Lists render with their structure

The system SHALL render unordered lists, ordered lists and description lists,
including nesting to the depth the source uses.

An unordered list item SHALL carry a marker; an ordered list item SHALL carry a
number reflecting its position; a description list SHALL present each term with
its description. Nested levels SHALL be indented relative to their parent.

A marker SHALL be set on the same line as the beginning of the text it marks,
so that an item reads as one thing rather than as a mark above a paragraph.

An ordered item's number SHALL be the one the renderer determined. Where the
source declares a starting number, the renderer SHALL count from it rather than
from one. The number SHALL NOT be inferred by the layout engine from the text it
was given, because a number the renderer did not choose is a number it cannot
guarantee.

A list continuation SHALL attach the block that follows it to the preceding
list item rather than ending the list.

#### Scenario: A nested unordered list is indented

- **WHEN** an unordered list contains a nested unordered list
- **THEN** both levels render as lists
- **AND** the nested level is indented relative to its parent

#### Scenario: An ordered list numbers its items

- **WHEN** an ordered list contains three items
- **THEN** the items are numbered in order

#### Scenario: An ordered list counts from a declared start

- **WHEN** an ordered list declares a starting number
- **THEN** its first item carries that number
- **AND** the items after it continue from there

#### Scenario: A marker sits beside the text it marks

- **WHEN** a list item holds a single short line of text
- **THEN** the marker and the first line of that text are set on one line
- **AND** this holds for unordered, ordered and description items alike

#### Scenario: An item's text does not start a line of its own

- **WHEN** a list item's text is short enough to fit beside its marker
- **THEN** no line of the output contains the marker and nothing else

#### Scenario: A description list pairs terms with descriptions

- **WHEN** a description list defines two terms
- **THEN** each term renders with its description

#### Scenario: A continuation keeps a block inside its item

- **WHEN** a list item is followed by a continuation and a further paragraph
- **THEN** that paragraph renders as part of the item
- **AND** the list continues afterwards rather than restarting

### Requirement: Breaks are honoured

The system SHALL render a thematic break as a visible division in the flow of
the text, and SHALL start a new page where the source asks for a page break.

#### Scenario: A page break starts a new page

- **WHEN** a document contains a page break between two paragraphs
- **THEN** the second paragraph begins on a later page than the first

#### Scenario: A thematic break divides the text

- **WHEN** a document contains a thematic break between two paragraphs
- **THEN** a visible division appears between them
- **AND** no page break results
### Requirement: A block title is rendered with its block

Where the source gives a block a title, the system SHALL render that title with
the block it belongs to, visually distinguished from both the block's content
and from body text.

#### Scenario: A titled block shows its title

- **WHEN** a block is preceded by a block title
- **THEN** the title renders immediately with that block
- **AND** it is distinguishable from the block's content

#### Scenario: A block title is not a heading

- **WHEN** a document contains a block title and a section heading
- **THEN** the two are presented differently
- **AND** the block title does not affect section nesting
### Requirement: Comments never reach the page

The system SHALL render neither single-line comments nor comment blocks, and
SHALL NOT report them as skipped constructs: a comment is content the author
asked to omit, not content the renderer failed to handle.

#### Scenario: A comment is absent from the output

- **WHEN** a document contains a single-line comment and a comment block
- **THEN** neither appears in the rendered text
- **AND** neither is named in the report
### Requirement: Markdown-compatible syntax is honoured

Where the AsciiDoc language accepts a Markdown-compatible spelling of a
construct this renderer supports, the system SHALL render it identically to the
AsciiDoc spelling.

#### Scenario: A Markdown heading is a section heading

- **WHEN** a document uses Markdown-style heading markers
- **THEN** the headings render at the same levels as the AsciiDoc spelling
  would produce

#### Scenario: A fenced code block is a listing block

- **WHEN** a document contains a fenced code block
- **THEN** its content renders verbatim, as a listing block would

### Requirement: Lists honour the presentation the source declares

The system SHALL honour the presentation attributes the AsciiDoc language
defines for lists: an unordered list's marker shape, a description list set
horizontally or as questions and answers, and a checklist's checked and
unchecked items.

A checklist item's marker SHALL show whether it is checked, and the marker
syntax SHALL NOT appear as text on the page.

Where a presentation attribute is not honoured, the list SHALL still render with
its default presentation and the attribute SHALL be reported, so that the
content is never lost to a presentation choice.

#### Scenario: A declared marker shape is used

- **WHEN** an unordered list declares a marker shape
- **THEN** its items carry that shape rather than the default

#### Scenario: A horizontal description list sets terms beside descriptions

- **WHEN** a description list is declared horizontal
- **THEN** each term is set beside its description rather than in front of it
- **AND** the terms align with one another

#### Scenario: A checklist shows what is checked

- **WHEN** a list contains a checked item and an unchecked item
- **THEN** each item's marker shows its state
- **AND** no bracket from the checklist syntax appears as text

#### Scenario: An unhonoured list attribute costs no content

- **WHEN** a list declares a presentation attribute this renderer does not
  honour
- **THEN** every item still renders
- **AND** the attribute is named in the skipped report

### Requirement: Paragraphs honour the presentation the source declares

The system SHALL honour the alignment a paragraph declares — left, centre,
right or justified — and SHALL set a paragraph declared as a lead paragraph
distinctly from body text.

An alignment a paragraph declares SHALL override the theme's justification
setting for that paragraph only, and SHALL NOT change how any other paragraph is
set.

#### Scenario: A centred paragraph is centred

- **WHEN** a paragraph declares centre alignment
- **THEN** its lines are centred on the measure
- **AND** the paragraphs around it are unaffected

#### Scenario: A justified paragraph is justified under an unjustified theme

- **WHEN** a paragraph declares justified alignment under a theme that does not
  justify
- **THEN** that paragraph is set flush to both margins
- **AND** the rest of the document remains ragged-right

#### Scenario: A lead paragraph is set apart

- **WHEN** a paragraph is declared a lead paragraph
- **THEN** it is set distinctly from the body text around it
