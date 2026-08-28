# Licensing

`adocpdf` is licensed **Apache-2.0**. The full text is in [`LICENSE`](LICENSE)
at the repository root.

That choice is constrained rather than free, and it is worth stating plainly so
it is not discovered later by someone preparing a release.

## The engine forces Apache-2.0

The layout engine is embedded, not shelled out to, so its licence applies to
anything distributed containing it:

| Crate | Version | Licence |
|---|---|---|
| `typst` | 0.15.1 | **Apache-2.0** |
| `typst-layout` | 0.15.1 | **Apache-2.0** |
| `typst-pdf` | 0.15.1 | **Apache-2.0** |

These are Apache-2.0 **only** — not the MIT/Apache dual licence common
elsewhere in the Rust ecosystem. Anyone redistributing `adocpdf`, in source or
binary form, inherits Apache-2.0 obligations from the engine. Relicensing
`adocpdf` under anything incompatible would mean replacing the engine.

## Other dependencies

Permissively licensed and compatible:

| Crate | Version | Licence | Where |
|---|---|---|---|
| `asciidoc-parser` | 0.29.19 | MIT OR Apache-2.0 | `adocpdf-asciidoc` |
| `clap` | 4.6.6 | MIT OR Apache-2.0 | `adocpdf-cli` |
| `thiserror` | 2.0.20 | MIT OR Apache-2.0 | `adocpdf-domain` |
| `toml` | 1.1.4 | MIT OR Apache-2.0 | `xtask` (tooling) |
| `pdf-extract` | 0.12.0 | MIT | `adocpdf-cli` (tests only) |

`adocpdf-core` has no third-party dependencies at all.

## The bundled font

`crates/adocpdf-typst/assets/fonts/DejaVuSans.ttf` is compiled into the binary,
so it is redistributed with every copy.

- **Licence:** Bitstream Vera (DejaVu changes are public domain).
- **Full text:** `crates/adocpdf-typst/assets/fonts/LICENSE-DejaVu.txt`.

The licence requires that **the copyright and permission notice be included with
every copy of the font software**. Because the font is embedded in the binary,
this means `LICENSE-DejaVu.txt` must be distributed alongside any build of
`adocpdf` — including binary-only releases.

It also forbids using the names "Bitstream" or "Vera" in the names of modified
versions of the font. We do not modify the font, so this does not currently
apply, but subsetting or renaming it later would need care.

## The policy is enforced, not just described

Everything above is checked mechanically by `cargo deny check licenses`, a job
in `scripts/ci/gate.sh`. The allow-list lives in [`deny.toml`](deny.toml) and is
exhaustive: a dependency whose licence is not on it fails the build, so adding
one is a deliberate, reviewed edit rather than something that arrives with a
transitive version bump.

Strong copyleft — GPL, AGPL, LGPL, MPL — is absent from the list by
construction. Two crates in the tree do name a copyleft licence, and both are
disjunctions offering a permissive alternative that the allow-list selects:

| Crate | Licence | Reached via |
|---|---|---|
| `self_cell` | `Apache-2.0 OR GPL-2.0-only` | `asciidoc-parser` |
| `r-efi` | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | UEFI targets only |

Neither obliges this project to anything copyleft. They are recorded so a future
reader does not rediscover them and assume something is wrong.

The allow-list was derived from a survey of the actual dependency tree, not from
expectation — which is how `Unicode-3.0` (27 crates, via the ICU stack the
layout engine pulls in for text segmentation) came to be on it.

## Before releasing

- Ship `LICENSE-DejaVu.txt` with every binary artefact.
- Ship [`LICENSE`](LICENSE), the Apache-2.0 text, and a NOTICE file if one is
  added.
- Re-check this file whenever a dependency is added — `architecture.toml` makes
  every new dependency an explicit edit, and `deny.toml` will reject an
  unfamiliar licence outright. Both are moments to record the change here.
