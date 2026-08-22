## MODIFIED Requirements

### Requirement: Supported document constructs

The system SHALL publish an inventory of the AsciiDoc language's constructs
recording, for each, whether this renderer honours it. The inventory SHALL
distinguish constructs that are honoured, constructs scheduled for later work,
and constructs that will never be honoured because they have no meaning on a
typeset page.

The system SHALL honour at least the constructs the inventory records as
honoured, and the inventory SHALL NOT record as honoured any construct the
system does not render.

Encountering a construct outside the honoured set SHALL NOT abort the render.
The system SHALL render the document without the unsupported construct and
SHALL report that the construct was skipped, naming it and its source location,
so the omission is never silent.

Where an unsupported *inline* construct carried text, that text SHALL still be
rendered: it is prose the author wrote inside a sentence, and dropping it would
leave a hole in a line. An unsupported *block* construct SHALL be reported by
name and location; whether its text is kept is decided per construct when that
construct is honoured, because the answer differs by construct — a footnote's
body is prose, while a table's cells re-flowed into the paragraph stream would
read as text the author never wrote, in an order they never chose.

Section headings SHALL be visually distinguishable from one another at every
level the renderer honours, not only at the first two. A reader who cannot tell
one level from another cannot recover the document's structure from the page,
which is the whole purpose of setting headings differently from body text.

A heading the author marks as discrete SHALL be set as a heading but SHALL take
no part in the section hierarchy: it SHALL NOT change the level of any heading
after it, and it SHALL NOT introduce a section that could contain the blocks
that follow.

#### Scenario: Nested sections keep their level

- **WHEN** the source contains a section and a subsection beneath it
- **THEN** both headings appear in the output
- **AND** they are visually distinguished from each other and from body text

#### Scenario: Deeper heading levels are told apart

- **WHEN** a document contains headings at every level the renderer honours
- **THEN** no two levels are set identically
- **AND** each level is distinguishable from body text

#### Scenario: A discrete heading is set but does not nest

- **WHEN** a section contains a discrete heading followed by a paragraph
- **THEN** the discrete heading is set as a heading
- **AND** the paragraph after it still belongs to the enclosing section
- **AND** a section heading following it keeps the level it would have had

#### Scenario: An unsupported construct is skipped, not fatal

- **WHEN** the source contains a construct the system does not yet support
- **THEN** the render succeeds and produces a PDF
- **AND** the report names the skipped construct and its location in the source

#### Scenario: An unsupported inline construct keeps its text

- **WHEN** a sentence contains an inline construct the system does not yet
  support
- **THEN** the text that construct carried still appears in that sentence
- **AND** the construct is named in the skipped report

#### Scenario: An unsupported block construct is reported rather than re-flowed

- **WHEN** a document contains a block construct the system does not yet
  support
- **THEN** the rest of the document renders unchanged
- **AND** the construct is named in the skipped report, with its location
- **AND** its content is not re-flowed into the surrounding paragraphs

#### Scenario: The inventory matches what the renderer does

- **WHEN** the inventory records a construct as honoured
- **THEN** a document using that construct renders it
- **AND** the construct is not named in the skipped report

#### Scenario: A permanently unsupported construct is not reported as pending

- **WHEN** the source contains a construct that has no meaning on a typeset
  page
- **THEN** the render succeeds and the construct is reported as skipped
- **AND** the inventory records it as never to be supported, rather than as
  scheduled work
