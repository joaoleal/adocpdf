## Context

See `proposal.md`. The finding came from the `parse_plan_emit` fuzz target
added by `behavioural-testing`; the reproducer is one byte and is kept at
`fuzz/artifacts/parse_plan_emit/timeout-1e32e3c3…`.

Attribution is settled, not assumed. A direct call to
`asciidoc_parser::Parser::parse("\u{c}")` — no safe mode, no reference time,
none of this project's code — does not return. The defect is upstream in
`asciidoc-parser` 0.29.19.

## Goals / Non-Goals

**Goals:**

- Make `adocpdf` terminate on every input, which `SECURITY.md` already promises.
- Refuse precisely the inputs that do not terminate, and nothing else.
- Say why, in the error, so an author can act on it.

**Non-Goals:**

- Fixing or vendoring `asciidoc-parser`.
- Sanitising input generally. See D3.
- Refusing on suspicion. See D1.

## Decisions

### D1 — The refused set is measured, not guessed

Every control character was probed against the real parser, alone and embedded
in text. The result is narrow and specific:

| Input | Outcome |
|---|---|
| `U+000B` alone, `U+000C` alone | **hangs** |
| `\u{c}\u{c}`, `\u{b}\u{c}` | **hangs** |
| `" \u{c}"`, `"\u{c} "`, `"\t\u{c}"`, `"\n\u{c}"`, `"\u{c}\n"` | **hangs** |
| `"a\u{c}"`, `"\u{c}a"`, `"Hello\u{c}world"`, `"a\n\u{c}"` | returns |
| `""`, `"   "`, `"\n"` | returns |
| every other C0, DEL and C1 character, alone or embedded | returns |

Two characters, and only in one situation. The rule the table describes is:

> A document fails to terminate if and only if it contains at least one
> `U+000B` or `U+000C` **and** contains no non-whitespace character.

That is the condition implemented. Not "contains a form feed", which would
refuse `"Hello\u{c}world"` — a document that parses today and must keep
parsing, since the proposal forbids changing what a working document does.

Guessing wider was rejected twice over: it would break working documents, and
a guard whose extent nobody measured is the kind of check `AGENTS.md`'s "Do not
write a checker" section is about.

### D2 — Refuse, do not strip

The alternative is to remove the offending characters and parse what is left.
Rejected. A whitespace-only document means nothing either way, so stripping
buys no working document; and silently altering input is a habit that is wrong
the first time it touches a document that did mean something. An error the
author can read is strictly better than a document they did not write.

### D3 — The guard lives in the adapter, not the domain

This is a fact about `asciidoc-parser` 0.29.19, not a rule about AsciiDoc. The
domain must not learn it: `adocpdf-domain` names no external technology, and a
constant listing two characters because of a bug in a particular version of a
particular parser is exactly the knowledge the layer boundary exists to keep
out.

So it goes in `AsciidocParser`, the adapter that owns the relationship with
that crate, and it maps to the existing `DomainError::ParseFailed` rather than
inventing a variant. From the domain's side nothing new has happened: a parser
declined a document and said why, which the port already models.

When upstream fixes this, the guard is deleted and the regression tests stay —
they will then be passing for the right reason instead of because of the
guard. That is the condition to revisit, and it is written into the code.

### D4 — The check is cheap and runs before the parser

A single pass over the source, tracking two booleans, before any parsing
begins. It cannot be after: the point is to not hand the input over.

Its cost is one scan of a document that is about to be fully parsed anyway, so
it does not need measuring against anything.

## Risks / Trade-offs

- **Upstream may fix this and the guard becomes dead code.** → Its comment
  names the version and the condition for removal, and the regression tests are
  independent of it, so removal is a small, verifiable edit rather than an
  archaeology exercise.

- **The rule could be subtler than the probe found.** The probe covered every
  C0, DEL and C1 character alone and embedded, plus whitespace combinations —
  not every possible document. → The fuzz target that found this runs on a
  schedule and keeps looking; a second hang would be a second finding, which is
  the arrangement working rather than failing.

- **Refusing a whitespace-only document is a behaviour change.** A document of
  nothing but spaces still parses; one containing a vertical tab now does not.
  → Both render nothing. The change is from "hangs forever" to "says no", and
  only for input that produced no document in the first place.

## Open Questions

- Whether `asciidoc-parser` has an upstream issue for this already, and what
  its maintainers consider the correct behaviour. Worth reporting; it changes
  nothing here, because the guard is needed until a fixed version is released
  and adopted.
