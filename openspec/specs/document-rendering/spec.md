# document-rendering Specification

## Purpose

Turning an AsciiDoc source document into a PDF file: what is read, what is
produced, which document constructs are honoured, how failures are reported,
and the guarantee that the same input always yields the same bytes.

## Requirements

### Requirement: Render an AsciiDoc file to a PDF file

The system SHALL accept a path to an AsciiDoc source file and a path for a PDF
output file, and SHALL produce a readable PDF document at the output path whose
content derives from the source.

The output SHALL be a well-formed PDF that a conforming reader can open. The
system SHALL NOT create or modify the output file when rendering fails; a failed
render leaves any pre-existing file at that path untouched.

#### Scenario: A minimal document renders

- **WHEN** the source contains a document title and one paragraph
- **THEN** a PDF file is created at the output path
- **AND** it contains at least one page
- **AND** the title and the paragraph text are both present in the rendered text

#### Scenario: The source file does not exist

- **WHEN** the input path names a file that is not present
- **THEN** rendering fails with an error identifying the missing input path
- **AND** no file is created at the output path

#### Scenario: Malformed source still produces a document

- **WHEN** the source contains malformed AsciiDoc — an unterminated block, a
  stray delimiter, or text matching no construct at all
- **THEN** rendering does not fail on that account
- **AND** a PDF is created containing whatever the source could be understood to
  mean
- **AND** anything that could not be represented is reported as skipped

#### Scenario: The output path cannot be written

- **WHEN** the output path is not writable
- **THEN** rendering fails with an error identifying the output path
- **AND** the failure is distinguishable from a failure to read the input and
  from a failure to lay the document out

### Requirement: Supported document constructs

The system SHALL honour the following constructs when rendering: the document
title, section headings and their nesting level, paragraphs, and inline text.

Encountering a construct outside that set SHALL NOT abort the render. The system
SHALL render the document without the unsupported construct and SHALL report
that the construct was skipped, naming it and its source location, so the
omission is never silent.

#### Scenario: Nested sections keep their level

- **WHEN** the source contains a section and a subsection beneath it
- **THEN** both headings appear in the output
- **AND** they are visually distinguished from each other and from body text

#### Scenario: An unsupported construct is skipped, not fatal

- **WHEN** the source contains a construct the system does not yet support
- **THEN** the render succeeds and produces a PDF
- **AND** the report names the skipped construct and its location in the source

### Requirement: Rendering is deterministic

Rendering the same source with the same configuration SHALL produce
byte-identical PDF output, regardless of when it runs, on which machine, under
which locale, or in which order internal collections happen to be traversed.

Any date embedded in the output SHALL come from a caller-supplied value rather
than from the host clock.

#### Scenario: Repeated renders match byte for byte

- **WHEN** the same source is rendered twice with the same configuration
- **THEN** the two output files are byte-identical

#### Scenario: The embedded date is supplied, not observed

- **WHEN** a document that displays the current date is rendered twice with the
  same supplied date but at different wall-clock times
- **THEN** both outputs show the supplied date
- **AND** the two output files are byte-identical

### Requirement: Source content cannot alter rendering instructions

Text drawn from the source document SHALL be treated as content only. No
sequence of characters in an AsciiDoc source SHALL be able to introduce,
terminate, or alter a rendering instruction, change page geometry, change
typography, or cause anything outside the source to be read.

#### Scenario: Layout-control syntax in body text stays literal

- **WHEN** a paragraph contains characters that are meaningful to the rendering
  layer's own markup syntax
- **THEN** those characters appear verbatim in the rendered text
- **AND** the document's layout is identical to the same document with those
  characters replaced by ordinary letters

#### Scenario: A crafted heading cannot escape into instructions

- **WHEN** a section heading contains text resembling a rendering directive
- **THEN** the heading renders as literal text
- **AND** no page geometry or typography change occurs as a result
