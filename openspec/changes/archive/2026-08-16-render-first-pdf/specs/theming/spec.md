## Purpose

Deciding which visual theme applies to which part of a document, so that
different sections can be rendered with different themes, and making the
page-break consequences of a theme change explicit rather than surprising.

## ADDED Requirements

### Requirement: A document has a default theme

Every document SHALL render under a theme. When no theme is specified, the
system SHALL apply a built-in default that produces a readable document
without any configuration.

A theme SHALL define page geometry (page size and margins) and typography (font
family, size, and line spacing) as separately identifiable groups of settings.

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

### Requirement: A section can override the theme

A section SHALL be able to declare a theme that differs from the document
default. The declared theme SHALL apply to that section and to its nested
subsections, unless a subsection declares its own.

Content outside any section declaring an override SHALL keep the document
default.

#### Scenario: An override applies to a section and its children

- **WHEN** a section declares a theme and contains a subsection that declares
  none
- **THEN** both the section and the subsection render under the declared theme

#### Scenario: A nested override wins over its parent

- **WHEN** a subsection declares a theme different from the one its parent
  section declares
- **THEN** the subsection renders under its own theme
- **AND** the parent's remaining content renders under the parent's theme

#### Scenario: An unknown theme is rejected

- **WHEN** a section declares a theme that is not defined
- **THEN** rendering fails with an error naming the unknown theme and the
  section that referenced it
- **AND** no output file is produced

### Requirement: A page-geometry change forces a page break

When a theme transition changes page geometry, the system SHALL begin the
newly themed content on a new page. When a theme transition changes only
typography, the system SHALL NOT force a page break, and the newly themed
content SHALL continue on the current page.

The system SHALL be able to report, for any theme transition, whether it forces
a page break — so an author can predict the effect before rendering.

#### Scenario: Changing page size breaks the page

- **WHEN** a section declares a theme whose page geometry differs from the
  theme in effect immediately before it
- **THEN** that section's content starts on a new page

#### Scenario: Changing only the font does not break the page

- **WHEN** a section declares a theme that differs from the previous theme only
  in typography
- **THEN** that section's content continues on the same page
- **AND** the new typography applies from that section onward

#### Scenario: A transition between identical themes changes nothing

- **WHEN** a section declares a theme whose settings all equal those of the
  theme already in effect
- **THEN** no page break occurs
- **AND** the rendered output is identical to the same document without the
  redundant declaration
