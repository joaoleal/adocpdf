## MODIFIED Requirements

### Requirement: Lists render with their structure

The system SHALL render unordered lists, ordered lists and description lists,
including nesting to the depth the source uses.

An unordered list item SHALL carry a marker; an ordered list item SHALL carry a
number reflecting its position; a description list SHALL present each term with
its description. Nested levels SHALL be indented relative to their parent.

A marker SHALL be set on the same line as the beginning of the text it marks,
so that an item reads as one thing rather than as a mark above a paragraph.

An ordered item's number SHALL be the one the renderer determined from the
item's position. It SHALL NOT be inferred by the layout engine from the text it
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
