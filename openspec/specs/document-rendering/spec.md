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

This SHALL hold for every channel by which source text reaches the renderer,
including content that arrives without substitution applied to it, and
including content that resembles the renderer's own internal encoding of
document structure.

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

#### Scenario: A marker the parser reserves cannot be typed by a document

- **WHEN** a document contains a character the parsing component reserves for
  its own internal markers
- **THEN** the document is refused with an error naming the character
- **AND** the refusal is bounded to the characters actually reserved, so
  ordinary private-use characters still render

#### Scenario: Unsubstituted content cannot escape into instructions

- **WHEN** source text reaches the renderer with no substitution applied to it,
  and that text resembles a rendering directive or the renderer's internal
  encoding of structure
- **THEN** it renders as literal text
- **AND** the document's layout is unchanged by its presence
### Requirement: Rendering terminates or refuses, on every input

The system SHALL return a result for every input it is given, whether that
result is a rendered document or an error. No input SHALL cause rendering to
continue indefinitely, and no input SHALL cause it to abort.

Where an underlying component cannot be relied on to process a particular input
without failing to terminate, the system SHALL refuse that input with an error
naming what was found and where, rather than passing it on. The error SHALL
make clear that the input was declined because a component cannot handle it,
not because the document is invalid.

A refusal MAY be wider than the input demonstrated not to terminate. Where the
condition for the defect cannot be stated reliably, the system SHALL prefer a
rule that cannot be outflanked — a rule about a character, rather than about
the shape it appears in — and SHALL bound and record what else that refuses. A
renderer that declines a malformed document is a nuisance; one that never
returns is a vulnerability.

A refusal SHALL NOT extend to input any ordinary authoring tool produces. In
particular, a document using carriage-return-and-line-feed line endings SHALL
render exactly as the same document with line-feed endings does.

Where an underlying component fails abruptly on an input — rather than failing
to terminate — and the inputs it fails on cannot be identified from the source
text, the system SHALL contain that failure and report it as a refusal, rather
than letting it end the process. A rule that reads the source is preferred
where one can be written; where the source cannot decide the question, the
system SHALL NOT ship a rule known to be incomplete in its place.

#### Scenario: A character the parser cannot be trusted with is refused

- **WHEN** a document contains a vertical tab or a form feed, anywhere in it
- **THEN** rendering fails with an error identifying the offending character
  and where it was found
- **AND** the command exits rather than continuing to run
- **AND** this holds whether or not the character sits beside other content

#### Scenario: A carriage return that cannot be parsed without hanging is refused

- **WHEN** a document contains a carriage return immediately followed by
  whitespace that is not a line feed
- **THEN** rendering fails with an error identifying what was found and where
- **AND** this holds whether or not the document has other content

#### Scenario: The refusal is bounded

- **WHEN** a document contains any other control character, or a carriage
  return followed by ordinary text, or one ending the document
- **THEN** it renders as it did before

#### Scenario: Ordinary Windows line endings are unaffected

- **WHEN** a document uses carriage-return-and-line-feed line endings
  throughout
- **THEN** it renders exactly as the same document with line-feed endings does
- **AND** no error is reported

#### Scenario: An ordinary empty document is unaffected

- **WHEN** a document is empty, or contains only spaces and newlines
- **THEN** it is accepted and renders as it did before

#### Scenario: A construct that makes the parser fail abruptly is refused

- **WHEN** a document contains an inline image or icon macro written without a
  target, as in `image:[alt]`
- **THEN** rendering fails with an error naming the document and the construct
  responsible
- **AND** the process ends through that error rather than aborting
- **AND** this holds even when the macro is assembled by attribute
  substitution and so appears nowhere in the source text

#### Scenario: Containing an abrupt failure does not refuse anything else

- **WHEN** a document contains an image or icon macro that names a target, or
  prose merely containing the word
- **THEN** it renders as it did before

#### Scenario: The refusal explains itself

- **WHEN** input is refused for this reason
- **THEN** the error names what was found and where
- **AND** the message distinguishes this from a syntax error in the document
