# adocpdf

Renders AsciiDoc to PDF, in Rust, with Typst embedded as the layout engine.

It renders inline formatting, the common block constructs and lists, and it
lays text out with an optimal line breaker rather than a greedy one. Most of
the AsciiDoc language is still ahead of it, though —
[`docs/asciidoc-support.md`](docs/asciidoc-support.md) says exactly which parts,
construct by construct, and that file is checked by a test rather than
maintained by hope.

## What works today

```bash
adocpdf book.adoc book.pdf
```

- Document title, section headings (nested), paragraphs.
- **Inline formatting**: bold, italic, monospace, superscript, subscript and
  highlight, nested to any depth, plus curved quotes, em dashes, ellipses,
  arrows and the other character replacements. Attribute references resolve;
  an undefined one is reported rather than silently emptied.
- **Blocks**: literal, listing, source and fenced code blocks set verbatim in a
  monospace face; all five admonitions in both their forms; quotes and verses
  with attribution; examples, sidebars and open blocks, nested; thematic and
  page breaks; block titles; comments.
- **Lists**: unordered, ordered and description lists, nested, with
  continuation.
- Per-section themes: a section can declare `[theme=wide]` or
  `[theme=large-print]` and the section plus everything nested inside it renders
  under that theme.
- A theme that changes **page geometry** starts a new page; one that changes
  only **typography** does not. That distinction is deliberate and tested.
- **Optimal line breaking.** Paragraphs are broken by optimising the paragraph
  as a whole rather than by filling each line in turn, whether or not the text
  is justified. Widow and orphan avoidance is on. Source wrapping does not reach
  the page: hard-wrap your `.adoc` however you like and the same prose comes out
  the same, because a newline inside a paragraph is the editor's and not the
  author's. A `+` marker and a verse block still break where they say.
- Reproducible output: `--date YYYY-MM-DD` makes the same source produce
  byte-identical bytes on every run and every machine.
- Everything read or written is confined to a project root, judged by where a
  path *resolves* — traversal, absolute paths and outward symlinks are all
  refused alike.
- Unsupported constructs are skipped and reported by name and source location,
  never silently dropped and never fatal. An unsupported *inline* construct
  still puts its text on the page, so a sentence never loses a phrase; an
  unsupported *block* is reported rather than re-flowed into the text around
  it.

```
adocpdf <INPUT> <OUTPUT> [--project-root DIR] [--date YYYY-MM-DD]
```

## What does not work yet

**[`docs/asciidoc-support.md`](docs/asciidoc-support.md) is the list**, one row
per construct the AsciiDoc language defines, saying whether this renderer
honours it, and which later change will. It is not prose that drifts: the rows
are read by a test that renders each sample and checks the claim, so a row
cannot say "honoured" without the renderer honouring it.

The headline gaps, all scheduled there:

- **Tables, cross-references, footnote bodies, images and includes.** The
  parser hands all of them over fully structured; rendering them is scheduled
  work, not a parsing problem.
- **No incremental rendering.** The whole document is recompiled every time.
  Whether partial *PDF* regeneration is achievable at all — as opposed to SVG
  preview of visible pages — is still an open question, and optimal line
  breaking makes it harder, since a change to one word can reflow a whole
  paragraph.
- **No theme file.** Themes are the built-in `wide` and `large-print` plus a
  default. Designing an authoring format is its own piece of work.
- **No WASM build.** `adocpdf-wasm` compiles but is empty. Whether the Typst
  dependency tree builds for `wasm32-wasip1` is untested.

Two things are recorded there as never coming: audio and video blocks, and
docinfo. A page cannot play sound, and there is no HTML head to inject into.

## Building

Requires the toolchain pinned in `rust-toolchain.toml` (1.97.1). `rustup` will
fetch it automatically, including the `llvm-tools-preview` component that
coverage needs.

```bash
cargo build --workspace
cargo run -p adocpdf-cli -- book.adoc book.pdf
```

### The quality gate

Run it before considering any change done:

```bash
scripts/ci/gate.sh
```

Its seventeen jobs need tools that do not ship with the toolchain:

```bash
cargo install cargo-llvm-cov cargo-audit cargo-deny cargo-machete \
              typos-cli taplo-cli cargo-hack zizmor --locked
sudo apt install shellcheck          # or your platform's package manager
rustup toolchain install 1.92 --profile minimal   # the declared MSRV
```

`actionlint` has no crates.io release; take its binary from
[the releases page](https://github.com/rhysd/actionlint/releases).

**A job whose tool is missing fails — it never skips.** A check that quietly
skips reports success on a machine that verified nothing, which is worse than
having no check at all. The failure message carries the install command.

| Job | Checks |
|---|---|
| formatting | `cargo fmt --check`, against the policy in `rustfmt.toml` |
| lints | `cargo clippy -D warnings` — `pedantic`, `nursery`, `cargo`, selected `restriction`, plus rustc's own groups |
| tests | the whole suite |
| architecture | dependencies flow inward, per `architecture.toml` |
| wasm build | the crates that must stay wasm-clean still compile |
| docs | `cargo doc` with warnings denied |
| msrv | the workspace really compiles on the declared 1.92 |
| shell | `shellcheck` on the gate script itself |
| toml | `taplo` formatting across every `.toml` |
| spelling | `typos` over source and documentation |
| feature combinations | `cargo hack` — every feature powerset compiles |
| workflow syntax | `actionlint` — expressions, runner labels, and shellcheck over `run:` blocks |
| workflow security | `zizmor` — script injection, over-broad permissions, unpinned actions |
| advisories | `cargo audit` against RUSTSEC |
| licences | `cargo deny` licence allow-list, bans, and sources |
| unused deps | `cargo machete` |
| coverage | **at least 90% line coverage**, workspace-wide |

Code quality is enforced entirely by these tools, configured in `rustfmt.toml`,
`clippy.toml`, `taplo.toml`, `_typos.toml` and `[workspace.lints]`. The project
writes no checker of its own except the architecture guard, which exists only
because the layer table is particular to this codebase and no tool can know it.

Coverage currently sits at about 96%. The floor is a fixed 90%, not a ratchet.

### Beyond the gate

Coverage says a line ran. It does not say the line is right, and it says
nothing about the inputs nobody thought to write down. Three instruments cover
what it misses:

- **Property tests** (`proptest`) — part of the ordinary suite, so they run in
  the gate. The two security-relevant rules are stated as properties rather
  than examples: everything reaching the output through
  `markup::string_literal` survives a round trip, no source text can produce a
  structural marker, and a path is judged by where it resolves rather than how
  it is spelled. Counterexamples are saved to `proptest-regressions/` and
  committed.
- **Fuzzing** (`cargo-fuzz`, weekly, **needs nightly**) — throws arbitrary
  bytes at the parse → plan → emit path looking for a panic or a hang. Every
  defect it has found so far has been in `asciidoc-parser` 0.29.19 rather than
  here: three non-terminating inputs — a vertical tab or form feed, and a
  carriage return followed by whitespace that is not a line feed, both of which
  hang even on a document with real content — and two panics, one of them an
  inline `image:` or `icon:` macro written with no target, which an author
  reaches by forgetting a filename. The hangs are refused before parsing; the
  panics cannot be predicted from the source text, so they are contained at the
  parse call and reported as errors. Every reproducer is a permanent test, and
  documents with Windows line endings are unaffected.

  The job does not fail on those two, and the reason is worth knowing:
  `libfuzzer-sys` aborts before unwinding, so the guard that contains them never
  runs under the fuzzer. `fuzz/known-crashes.toml` records each one with its
  reason, `scripts/ci/known-crashes.sh` fails only on a reproducer that is not
  recorded — matching by exact bytes and printing what it tolerated — and a test
  on the pinned toolchain asserts every entry is still refused, so an entry
  cannot outlive the defect it names.
- **Mutation testing** (`cargo-mutants`, weekly) — breaks the code on purpose
  and checks a test notices. Enforced on the injection boundary, the inline
  decoder and the sandbox rule; reported without a threshold everywhere else.

Neither fuzzing nor mutation testing is part of `scripts/ci/gate.sh`, and a
green gate does not claim otherwise. Both take far longer than a merge should
wait, and fuzzing cannot run on the pinned toolchain at all.

Two configuration files are worth knowing about. `deny.toml` holds the licence
allow-list — strong copyleft is absent by construction, so a dependency
offering only GPL or MPL fails the build. `.cargo/audit.toml` records the two
advisories that cannot be fixed from here, with the argument for tolerating
them and the condition that should end it; every other advisory still fails.

## How it fits together

```
AsciiDoc source
  → asciidoc-parser (infra)      parse, in the most restrictive safe mode
  → document model  (core)       title, sections, paragraphs
  → layout plan     (domain)     themes resolved, page breaks classified
  → Typst markup    (infra)      all content emitted as string literals
  → embedded Typst  (infra)      compiled in-process, from an in-memory World
  → PDF bytes
```

Dependencies flow strictly inward and a checked-in guard fails the build if they
do not. `adocpdf-core` has no dependencies at all; `adocpdf-domain` names no
external technology. See `AGENTS.md` for the layer table and the conventions.

Two properties worth knowing:

- **Source content can never become a rendering instruction.** Everything
  derived from the document is emitted as a Typst *string literal*, so the
  escaping surface is the quote, the backslash and control characters — a fixed
  set — rather than the whole markup grammar.
- **Fonts are embedded, not discovered.** Searching the host for fonts would
  make output machine-dependent, and would find nothing under WASM.

## Licensing

`adocpdf` is Apache-2.0. That is not a free choice: the Typst engine it embeds
is Apache-2.0 only, and anyone redistributing this software inherits those
obligations. The bundled DejaVu Sans font is under the Bitstream Vera licence,
whose permission notice must travel with every copy — see
`crates/adocpdf-infra/assets/fonts/LICENSE-DejaVu.txt`. Full details in
`LICENSING.md`.

## Development

This project uses [OpenSpec](https://openspec.dev/) for spec-driven development:
propose → apply → archive. Planning artifacts live in `openspec/changes/`. No
project code is written before a change proposal covering it exists.
