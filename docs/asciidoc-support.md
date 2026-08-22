# AsciiDoc support

Every construct the AsciiDoc language defines, and what `adocpdf` does with it.

This file is the scope of every later change, and it is **executable**: the
`Status` and `Sample` columns are read by `crates/adocpdf-infra/tests/support_inventory.rs`,
which renders each sample and checks the claim. A row cannot say `honoured`
without the renderer honouring it, and a row cannot say `partial` without
naming a later tier that finishes it. A document that promises more than the
code delivers is worse than no document, so the promise is tested.

## How to read it

| Column | Meaning |
|---|---|
| **Parser** | Whether `asciidoc-parser` 0.29.19 supports it. It supports nearly everything: its conformance suite mirrors the official language documentation with 1133 verified claims across all sixteen chapters. A gap below is almost always a *rendering* gap. |
| **Status** | `honoured` — rendered. `partial` — rendered incompletely, with what is missing named. `scheduled` — reported as skipped, and planned. `never` — reported as skipped, and not planned, with a reason. |
| **Tier** | Which change honours it. Tier 1 is done. |
| **Sample** | Minimal source proving the claim, with `\n` for a line break. Required for `honoured` and `partial`; `—` otherwise. |

Skipping is never silent: an unsupported construct is reported by name and
source location. An unsupported construct used *inline* keeps its text, so a
sentence never loses a phrase. An unsupported *block* is reported and left out
rather than re-flowed into the paragraphs around it — a table's cells poured
into the text stream would read as prose the author never wrote. Whether each
block keeps its content is settled when that construct is honoured; the tier
column says when that is.

## Inline text

| Construct | Syntax | Parser | Status | Tier | Sample |
|---|---|---|---|---|---|
| Bold | `*x*` `**x**` | yes | honoured | 1 | `A bold *word*.` |
| Italic | `_x_` `__x__` | yes | honoured | 1 | `An italic _word_.` |
| Monospace | `` `x` `` | yes | honoured | 1 | `A mono \`word\`.` |
| Highlight | `#x#` | yes | honoured | 1 | `A marked #word#.` |
| Superscript | `^x^` | yes | honoured | 1 | `E = mc^2^.` |
| Subscript | `~x~` | yes | honoured | 1 | `H~2~O.` |
| Nested formatting | `*_x_*` | yes | honoured | 1 | `Both *_at once_*.` |
| Curved quotes | `` "`x`" `` | yes | honoured | 1 | `She said "\`hello\`".` |
| Character replacements | `(C)` `--` `...` `->` | yes | honoured | 1 | `(C) 2026 -- see p. 3 ... a -> b.` |
| Typographic apostrophe | `it's` | yes | honoured | 1 | `It's here.` |
| Special characters | `<` `>` `&` | yes | honoured | 1 | `a < b & c > d.` |
| Character reference | `&#8212;` | yes | honoured | 1 | `An em dash &#8212; here.` |
| Hard line break | `+` at end of line | yes | honoured | 1 | `line one +\nline two` |
| Soft wrap (filled) | a newline inside a paragraph | yes | honoured | 1 | `a wrapped\nparagraph` |
| Attribute reference | `{name}` | yes | honoured | 1 | `:product: adocpdf\n\nBuilt with {product}.` |
| Escape | `\*x*` | yes | honoured | 1 | `Not \*bold* here.` |
| Inline passthrough | `+x+` `pass:[x]` | yes | partial (tier 3) | 1 | `Verbatim +*text*+ here.` |
| Custom role span | `[.role]#x#` | yes | scheduled | 2 | — |
| Underline, strikethrough | `[.underline]#x#` | yes | scheduled | 2 | — |
| Counters | `{counter:n}` | yes | scheduled | 5 | — |

**Inline passthrough** is `partial`: content is set verbatim, which is what the
author asked for, but a literal HTML entity spelling inside a
no-substitution passthrough (`+++&lt;+++`) decodes to the character. The
parser hands over no offsets for passthrough regions, so the decoder cannot
tell that entity from one the parser produced. Tier 3 revisits it.

**A soft wrap is not a break.** A newline inside a paragraph is where the
author's editor wrapped the source, and the renderer fills the paragraph to the
measure the theme gives it: the same prose renders identically whether its
source is hard-wrapped or written on one long line. Only the `+` marker, and a
verse block, put a line ending on the page.

## Blocks

| Construct | Syntax | Parser | Status | Tier | Sample |
|---|---|---|---|---|---|
| Paragraph | (implicit) | yes | honoured | 1 | `Just a paragraph.` |
| Literal paragraph | (indented) | yes | honoured | 1 | `  indented text` |
| Literal block | `....` | yes | honoured | 1 | `....\nas typed\n....` |
| Listing block | `----` | yes | honoured | 1 | `----\ncode here\n----` |
| Fenced code block | ` ``` ` | yes | honoured | 1 | "```\ncode here\n```" |
| Source block | `[source,lang]` | yes | honoured | 1 | `[source,rust]\n----\nfn main() {}\n----` |
| Passthrough block | `++++` | yes | honoured | 1 | `++++\nliteral\n++++` |
| Block title | `.Title` | yes | honoured | 1 | `.A Title\n----\ncode\n----` |
| Comment | `//` `////` | yes | honoured | 1 | `// a comment\n\nText.` |
| Note admonition | `NOTE:` | yes | honoured | 1 | `NOTE: Worth knowing.` |
| Tip admonition | `TIP:` | yes | honoured | 1 | `TIP: Try this.` |
| Important admonition | `IMPORTANT:` | yes | honoured | 1 | `IMPORTANT: Do not miss.` |
| Caution admonition | `CAUTION:` | yes | honoured | 1 | `CAUTION: Careful.` |
| Warning admonition | `WARNING:` | yes | honoured | 1 | `WARNING: Mind the gap.` |
| Delimited admonition | `[NOTE]` + `====` | yes | honoured | 1 | `[NOTE]\n====\nFirst.\n\nSecond.\n====` |
| Quote block | `[quote]` + `____` | yes | honoured | 1 | `[quote,A Name,A Work]\n____\nThe words.\n____` |
| Verse block | `[verse]` + `____` | yes | honoured | 1 | `[verse]\n____\nline one\nline two\n____` |
| Example block | `====` | yes | honoured | 1 | `====\nInside.\n====` |
| Sidebar | `****` | yes | honoured | 1 | `****\nAside.\n****` |
| Open block | `--` | yes | honoured | 1 | `--\nGrouped.\n--` |
| Thematic break | `'''` | yes | honoured | 1 | `Before.\n\n'''\n\nAfter.` |
| Page break | `<<<` | yes | honoured | 1 | `Before.\n\n<<<\n\nAfter.` |
| Markdown heading | `#` … `######` | yes | honoured | 1 | `== A heading\n\nBody.` |
| Markdown blockquote | `> quote` | yes | honoured | 1 | `> A quoted line.` |
| Collapsible block | `[%collapsible]` | yes | scheduled | 4 | — |
| Roles, IDs, options | `[#id.role]` | yes | scheduled | 2 | — |
| Paragraph alignment | `[.text-left]` | yes | scheduled | 2 | — |
| Lead paragraph | `[.lead]` | yes | scheduled | 2 | — |
| Source highlighting | `:source-highlighter:` | yes | scheduled | 5 | — |
| Callouts | `<1>` | yes | scheduled | 5 | — |

## Lists

| Construct | Syntax | Parser | Status | Tier | Sample |
|---|---|---|---|---|---|
| Unordered list | `* item` | yes | honoured | 1 | `* one\n* two` |
| Nested unordered list | `** item` | yes | honoured | 1 | `* one\n** nested\n* two` |
| Ordered list | `. item` | yes | honoured | 1 | `. first\n. second` |
| Nested ordered list | `.. item` | yes | honoured | 1 | `. first\n.. nested\n. second` |
| Description list | `term:: text` | yes | honoured | 1 | `apple:: a fruit\npear:: another` |
| List continuation | `+` | yes | honoured | 1 | `* one\n+\nStill the first item.\n\n* two` |
| Custom marker, start | `[circle]` `[start=4]` | yes | scheduled | 2 | — |
| Horizontal, Q&A lists | `[horizontal]` `[qanda]` | yes | scheduled | 2 | — |
| Checklist | `* [x]` | yes | scheduled | 2 | — |

## Sections and document structure

| Construct | Syntax | Parser | Status | Tier | Sample |
|---|---|---|---|---|---|
| Document title | `= Title` | yes | honoured | 1 | `= The Title\n\nBody.` |
| Formatted title | `= *Bold* Title` | yes | honoured | 1 | `= A *Bold* Title\n\nBody.` |
| Section titles, levels 1–5 | `==` … `======` | yes | honoured | 1 | `== One\n\nBody.\n\n=== Two\n\nMore.` |
| Attribute entry | `:name: value` | yes | honoured | 1 | `:name: value\n\nText.` |
| Preamble | (implicit) | yes | honoured | 1 | `= Title\n\nPreamble text.\n\n== Section\n\nBody.` |
| Discrete heading | `[discrete]` | yes | scheduled | 2 | — |
| Auto and custom IDs | `[#id]` | yes | scheduled | 2 | — |
| Section numbering | `:sectnums:` | yes | scheduled | 2 | — |
| Parts and chapters | `:doctype: book` | yes | scheduled | 2 | — |
| Special sections | `[appendix]` `[glossary]` | yes | scheduled | 2 | — |
| Author and revision lines | `Name <email>` | yes | scheduled | 2 | — |
| Subtitle | `= Title: Subtitle` | yes | scheduled | 2 | — |
| Table of contents | `:toc:` | yes | scheduled | 2 | — |
| Doctype | `:doctype:` | yes | scheduled | 2 | — |

## References

| Construct | Syntax | Parser | Status | Tier | Sample |
|---|---|---|---|---|---|
| Footnote | `footnote:[x]` | yes | partial (tier 3) | 1 | `Text.footnote:[a note]` |
| Autolink | `https://x` | yes | scheduled | 3 | — |
| URL and link macros | `https://x[t]` `link:p[t]` | yes | scheduled | 3 | — |
| Mailto | `mailto:a@b[t]` | yes | scheduled | 3 | — |
| Inline anchor | `[[id]]` | yes | scheduled | 3 | — |
| Cross-reference | `<<id>>` | yes | scheduled | 3 | — |
| Inter-document xref | `xref:d.adoc#id[t]` | yes | scheduled | 3 | — |
| Bibliography reference | `[[[a]]]` | yes | scheduled | 3 | — |
| Index term | `((term))` | yes | scheduled | 5 | — |

**Footnote** is `partial`: the marker is rendered and the construct reported,
but the note's body does not reach the page. It is not reachable from where
inline content is decoded — upstream documents `FootnoteRenderParams::text` as
carrying the unresolved ID and being "ignored in the other cases", because a
resolved footnote's content lives in the document's catalog. Tier 3 renders
footnote bodies properly.

## Tables

| Construct | Syntax | Parser | Status | Tier | Sample |
|---|---|---|---|---|---|
| Table | `\|===` | yes | scheduled | 3 | — |
| Column specifiers | `[cols="1,2"]` | yes | scheduled | 3 | — |
| Header and footer rows | `[%header]` `[%footer]` | yes | scheduled | 3 | — |
| Cell spans | `2+\|` | yes | scheduled | 3 | — |
| Cell alignment and style | `^\|` `>\|` `m\|` | yes | scheduled | 3 | — |
| CSV, TSV, DSV data | `format=csv` | yes | scheduled | 3 | — |
| Nested tables | `a\|` | yes | scheduled | 3 | — |

The parser hands tables over fully structured — columns, spans, alignments,
frames, header and footer rows, four data formats. Tier 3 is a rendering
problem, not a parsing one.

## Media and includes

| Construct | Syntax | Parser | Status | Tier | Sample |
|---|---|---|---|---|---|
| Block and inline image | `image::p[]` `image:p[]` | yes | scheduled | 4 | — |
| Image sizing, alt, link | `[alt,w,h]` | yes | scheduled | 4 | — |
| SVG images | `[opts=inline]` | yes | scheduled | 4 | — |
| `:imagesdir:`, `:data-uri:` | attribute | yes | scheduled | 4 | — |
| Include directive | `include::p[]` | yes | scheduled | 4 | — |
| Icons | `icon:name[]` | yes | scheduled | 5 | — |
| **Audio** | `audio::f[]` | yes | **never** | — | — |
| **Video** | `video::f[]` | yes | **never** | — | — |

**Audio and video are never supported.** A page cannot play sound or motion.
They are reported as skipped, and that will not change.

## Directives and other syntax

| Construct | Syntax | Parser | Status | Tier | Sample |
|---|---|---|---|---|---|
| Conditionals | `ifdef::` `ifeval::` | yes | scheduled | 5 | — |
| STEM | `stem:[]` `[stem]` | yes | scheduled | 5 | — |
| UI macros | `kbd:[]` `btn:[]` `menu:[]` | yes | scheduled | 5 | — |
| **Docinfo** | `docinfo.html` | yes | **never** | — | — |

**Docinfo is never supported.** It injects markup into an HTML document's head
and body; there is no such thing in a PDF.

## What the sandbox refuses regardless

Three rules sit outside this table and apply to every tier. Everything read or
written stays inside the project root, judged by where a path resolves rather
than how it is spelled — so an include pointing outside the root is refused
however it is scheduled here. And a document containing a Unicode noncharacter
(U+FDD0–U+FDEF) is refused outright: those characters are how inline structure
is marked internally, and accepting one would let a document forge formatting
it did not write. The four characters the parser reserves for its own
placeholders (U+E000–U+E003) are refused for the same reason; other private-use
characters, such as the icon-font glyphs real documents contain, still render.

Third, two kinds of input are refused because the parser does not return from
them, both found by fuzzing: a vertical tab or a form feed anywhere in the
document, and a carriage return immediately followed by whitespace that is not
a line feed. Both refusals are deliberately wider than the defect — some inputs
matching them do render — because the exact condition is not stable enough to
encode. **Windows line endings are unaffected**: in CRLF text every carriage
return is followed by a line feed.

Fourth, a document is refused if the parser fails abruptly while reading it.
Two cases are known. An inline `image:` or `icon:` macro written without a
target — `image:[alt]`, or the fuzzer's `image:[]` — panics inside the parser's
macro substitution. And two consecutive block attribute lines, the first
holding a `%` option shorthand followed by whitespace and another `%`, trip a
debug assertion; that one fires only in a debug build, so such a document is
refused there and renders in a release build. Unlike the three rules above,
this one reads nothing: attribute references are substituted before macros, so the macro need
not appear in the source at all, and no rule about the source text could decide
it. The failure is contained at the parse call and reported as an ordinary
error instead. An image macro that names a target is unaffected, and still
renders as a skipped construct until tier 4.
