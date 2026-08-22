## MODIFIED Requirements

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

## ADDED Requirements

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
