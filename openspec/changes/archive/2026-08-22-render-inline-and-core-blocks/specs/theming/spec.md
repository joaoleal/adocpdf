## MODIFIED Requirements

### Requirement: A document has a default theme

Every document SHALL render under a theme. When no theme is specified, the
system SHALL apply a built-in default that produces a readable document
without any configuration.

A theme SHALL define page geometry (page size and margins) and typography as
separately identifiable groups of settings. Typography SHALL name a body family
and a monospace family, together with size and line spacing, so that verbatim
and monospaced text has a face to be set in that is distinct from body text.

A theme SHALL NOT name a font family for which no face is available. A theme
naming an unavailable family SHALL be rejected rather than silently falling
back to another face, because a silent fallback would make output depend on
which faces happen to be present.

#### Scenario: An unthemed document still renders

- **WHEN** a document is rendered with no theme specified
- **THEN** the render succeeds using the built-in default theme
- **AND** the output has a defined page size and readable body text

#### Scenario: A theme with invalid values is rejected

- **WHEN** a theme specifies a non-positive page dimension, a non-positive font
  size, or margins that leave no printable area
- **THEN** the theme is rejected before rendering begins
- **AND** the error identifies which setting is invalid and why
- **AND** no output file is produced

#### Scenario: Monospaced text is set in the theme's monospace family

- **WHEN** a document contains monospaced inline text or a verbatim block
- **THEN** that text is set in the theme's monospace family
- **AND** it is visually distinguishable from body text

#### Scenario: A theme naming an unavailable family is rejected

- **WHEN** a theme names a font family for which no face is available
- **THEN** the theme is rejected before rendering begins
- **AND** the error names the family that could not be provided
- **AND** no output file is produced

## ADDED Requirements

### Requirement: Text is laid out with optimal line breaking

The system SHALL break lines by optimising each paragraph as a whole, rather
than by filling each line greedily in turn. This SHALL hold whether or not the
text is justified.

A theme SHALL be able to specify whether text is justified, the language the
text is written in, and how strongly widowed and orphaned lines are to be
avoided. Justification and language SHALL be independent of the choice of line
breaker: a document set ragged-right SHALL still be broken optimally.

Every one of these settings SHALL be carried by the theme and SHALL NOT be read
from the environment, so that identical inputs continue to produce identical
output.

#### Scenario: A paragraph is broken as a whole

- **WHEN** a paragraph is long enough that greedy and optimal line breaking
  would differ
- **THEN** the resulting line breaks are those of the optimal breaker
- **AND** this holds for an unjustified document as well as a justified one

#### Scenario: A theme can justify text

- **WHEN** a theme specifies justified text
- **THEN** the rendered paragraphs are set flush to both margins
- **AND** a theme that does not specify it renders ragged-right

#### Scenario: Hyphenation follows the theme's language

- **WHEN** a theme specifies justified text and names a language
- **THEN** words are hyphenated according to that language
- **AND** a theme naming no language produces no hyphenation

#### Scenario: Widowed and orphaned lines are avoided

- **WHEN** a paragraph would otherwise leave a single line stranded at the foot
  or head of a page
- **THEN** the layout avoids stranding it, according to the theme's setting

#### Scenario: Layout settings do not come from the environment

- **WHEN** the same document is rendered twice under different host locales and
  environments
- **THEN** the two outputs are byte-identical
