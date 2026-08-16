# Security Policy

`adocpdf` reads AsciiDoc files and writes PDFs. Its input is untrusted by
definition — the whole point of the tool is to typeset documents somebody else
wrote — so this file states what the project claims, so that a reporter can tell
a vulnerability from a missing feature.

## Supported versions

None yet. There has been no release. Only the `main` branch is supported, and
the current version is `0.1.0`, described in `README.md` as a walking skeleton.

## What counts as a vulnerability

Four boundaries. Each is tested, and each is documented in `AGENTS.md` or
`README.md`.

### 1. Source content must never become a rendering instruction

Everything derived from an AsciiDoc source reaches the output through
`markup::string_literal` in `crates/adocpdf-infra/src/markup.rs`, which emits it
as a Typst string literal in code mode rather than as markup. The escaping
surface is therefore the quote, the backslash and control characters — a fixed
set — rather than the whole markup grammar.

**Report it if** you can craft a source document whose content introduces,
terminates or alters a rendering instruction, changes page geometry or
typography, or causes anything outside the source to be read.

### 2. File access must stay inside the project root

Every path the renderer reads or writes must resolve inside the declared project
root. The containment rule lives in `adocpdf-domain::sandbox`; resolution goes
through the `PathResolver` port. Paths are judged by where they **resolve**, not
by how they are spelled, so traversal segments, absolute paths and symbolic
links pointing outward are all refused alike.

**Report it if** you can read or write a file outside the project root — through
an invocation argument, a directive inside a document, or a link.

### 3. Untrusted input must not panic or hang

The specification requires that malformed source not be fatal: unsupported
constructs are skipped and reported, never silently dropped and never fatal.

**Report it if** any input makes `adocpdf` panic, abort, fail to terminate, or
consume unbounded memory. This holds whether or not the input is valid
AsciiDoc — a crash on a malformed file is still a crash.

Note that as of today this property is tested by examples rather than by
fuzzing. Closing that gap is a planned change (`behavioural-testing`), and
until it lands, reports in this category are especially useful.

### 4. Findings in embedded dependencies go upstream

`adocpdf` statically embeds the Typst layout engine and the `asciidoc-parser`
crate. A vulnerability in either is reported to that project, not here. This
project tracks such findings with `cargo audit` as a mandatory gate job, and
records in `.cargo/audit.toml` any advisory it knowingly tolerates, with the
argument for tolerating it and the condition that should end it.

**Report it here** if an upstream weakness is reachable through `adocpdf` in a
way the upstream project would not consider its own problem — for example, if
this project's configuration of the engine is what exposes it.

## What is not a vulnerability

- **Unsupported AsciiDoc constructs.** Tables, lists, includes, admonitions,
  cross-references, images, callouts and inline formatting are not implemented.
  `README.md` lists them. They are skipped and reported, which is the specified
  behaviour, not a failure.
- **Rendering that looks wrong.** Layout defects are ordinary bugs.
- **Anything requiring the attacker to already control the machine**, the
  project root's contents, or the command line.

## Reporting

Use **GitHub private vulnerability reporting** on
<https://github.com/joaoleal/adocpdf> — the *Security* tab, *Report a
vulnerability*.

> **Note:** private reporting must be enabled in the repository's settings
> before that form appears. If it is not yet available, open a regular issue
> saying only that you have a security report and asking for a contact — do not
> put the details in a public issue.

Please include the source document or input that triggers it, the command line
used, and what you expected instead. A minimal reproducing `.adoc` file is worth
more than a description.

There is no bounty, and no guaranteed response time: this is a personal project.
You will get an acknowledgement and an honest answer about whether and when it
will be fixed.

## Disclosure

Report privately first. Once a fix is released, or if the project has not
responded within 90 days, you are welcome to disclose publicly. Credit is given
in the release notes unless you ask otherwise.
