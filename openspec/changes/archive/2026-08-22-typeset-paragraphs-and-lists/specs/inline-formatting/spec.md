## MODIFIED Requirements

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
