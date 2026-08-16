## Purpose

Confining every file the renderer reads or writes to a declared project root,
so that neither an invocation argument nor content inside a source document can
reach files elsewhere on the host.

## ADDED Requirements

### Requirement: All file access is confined to a project root

Rendering SHALL operate against a declared project root. Every path the system
reads or writes SHALL resolve to a location inside that root.

A path that resolves outside the root SHALL be rejected before any file
operation occurs. Rejection SHALL be based on where the path actually resolves
to, not on how it is spelled, so that traversal segments, absolute paths, and
symbolic links pointing outward are all refused.

The error SHALL report that the path lies outside the project root, and SHALL
NOT disclose the contents or existence of the out-of-root target.

#### Scenario: A path inside the root is accepted

- **WHEN** a source file inside the project root is rendered
- **THEN** the file is read and rendering proceeds

#### Scenario: A traversal path is refused

- **WHEN** a path containing parent-directory segments resolves outside the
  project root
- **THEN** the path is rejected before any read is attempted
- **AND** the error states that the path is outside the project root

#### Scenario: A symbolic link pointing outward is refused

- **WHEN** a path inside the root is a symbolic link whose target resolves
  outside the root
- **THEN** the path is rejected before the target is read
- **AND** the error does not reveal the target's location or contents

#### Scenario: An absolute path outside the root is refused

- **WHEN** an absolute path outside the project root is supplied as input or
  output
- **THEN** it is rejected before any file operation occurs

### Requirement: Document content cannot widen file access

Paths originating from inside a source document SHALL be subject to the same
confinement as paths supplied at invocation. No directive within a document
SHALL be able to read a file outside the project root, change the project root,
or cause a network access.

#### Scenario: An include pointing outside the root is refused

- **WHEN** a source document references a file that resolves outside the
  project root
- **THEN** the referenced file is not read
- **AND** the failure identifies the offending reference and its location in
  the source

#### Scenario: A remote reference is not fetched

- **WHEN** a source document references a location that is not a local file
  inside the project root
- **THEN** no network request is made
- **AND** the failure identifies the offending reference

### Requirement: The project root is explicit

The system SHALL determine the project root from an explicit caller-supplied
value, or, when none is given, from a documented default derived from the input
file's location. The system SHALL report the root in effect when it rejects a
path, so the confinement boundary is never ambiguous.

#### Scenario: The effective root appears in a rejection

- **WHEN** a path is rejected for lying outside the project root
- **THEN** the error names the root that was in effect

#### Scenario: A non-existent root is refused

- **WHEN** a project root is supplied that does not exist or is not a directory
- **THEN** rendering fails before any file is read
- **AND** the error identifies the supplied root
