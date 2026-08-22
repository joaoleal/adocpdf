## ADDED Requirements

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
