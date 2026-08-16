## ADDED Requirements

### Requirement: Rendering terminates on every input

The system SHALL return a result for every input it is given, whether that
result is a rendered document or an error. No input SHALL cause rendering to
continue indefinitely.

Where an underlying component cannot process a particular input without
failing to terminate, the system SHALL refuse that input with an error naming
what was found, rather than passing it on.

The refusal SHALL be limited to input demonstrated not to terminate. Input that
renders successfully SHALL continue to render identically.

#### Scenario: A document that cannot be parsed without hanging is refused

- **WHEN** a document consists only of whitespace and contains a vertical tab
  or a form feed
- **THEN** rendering fails with an error identifying the offending character
- **AND** the command exits rather than continuing to run

#### Scenario: The same character inside real content is not refused

- **WHEN** a document contains a vertical tab or a form feed alongside other
  content
- **THEN** the document renders as it did before
- **AND** no error is reported for the control character

#### Scenario: An ordinary empty document is unaffected

- **WHEN** a document is empty, or contains only spaces and newlines
- **THEN** it is accepted and renders as it did before

#### Scenario: The refusal explains itself

- **WHEN** input is refused for this reason
- **THEN** the error names the character by its Unicode code point
- **AND** the message distinguishes this from a syntax error in the document
