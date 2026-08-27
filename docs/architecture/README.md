# Architecture model

A [LikeC4](https://likec4.dev) model of this workspace: the nine crates, the ports the domain owns, the adapters that implement them, and the path a render takes.

## Viewing it

```bash
make -C docs/architecture architecture         # one self-contained HTML page
make -C docs/architecture architecture-serve   # dev server, reloads as you edit
make -C docs/architecture architecture-site    # static site, for hosting
make -C docs/architecture architecture-clean   # remove the output
make -C docs/architecture                      # list the targets
```

Run them from this directory without the `-C` if you are already here. Output lands in `build/` beside these sources and is ignored by git; the single-file page opens straight from the filesystem — no server, no network.

The renderer runs through `npx` at a version pinned in the `Makefile`, so nothing is installed into the workspace and the page does not change under you.

## Files

| File | What it holds |
| --- | --- |
| `spec.c4` | The vocabulary — element kinds, tags, relationship kinds |
| `model.c4` | The crates, ports, adapters, and the edges between them |
| `views.c4` | The views |
| `Makefile` | Rendering the above |

## The views

The first three are the C4 levels, in zoom order:

- **`index`** — *level 1, context.* Who uses adocpdf, and what it touches from outside.
- **`layers`** — *level 2, containers.* The nine crates and how they may depend on each other. A crate is this system's unit of containment.
- **`domain`**, **`asciidoc`**, **`typst`** and **`host`** — *level 3, components.* The six ports and the use case that drives them; then the inside of each crate that implements them.

There is no level 4. `crates/` is the code, and it never goes stale.

Then two that are not levels:

- **`rings`** — which of Clean Architecture's four rings each crate sits in. Every crate carries exactly one.
- **`render`** — one run of `adocpdf`, step by step, as a sequence.

## Reading the crate graph

Edges between crates come in three kinds, and the differences are the interesting part:

- **Solid (`depends`)** — declared in a manifest's `[dependencies]`. There are sixteen.
- **Dashed amber (`test`)** — declared only in `[dev-dependencies]`. There is one: `adocpdf-cli` on `adocpdf-core`, because the layout tests assert on the document model that nothing under `src/` names.
- **Dotted (`permitted`)** — allowed by `architecture.toml` and taken up by no manifest. There are five, four of them `adocpdf-wasm`'s, which declares nothing at all.

`architecture.toml` sets a ceiling rather than describing what exists: the guard in `xtask/tests/architecture.rs` fails a build whose manifest declares something the file does not permit, and never the reverse. So the dotted edges are headroom — room the rule leaves for a crate to grow into.

The dashed edge is worth its own kind because the guard cannot see the distinction. Dependencies and dev-dependencies are read into one list, so an edge taken for a test is permitted for production too, and nothing would report it if a future commit used it there. Drawing it apart is the only place that difference is visible.

## What the outer ring is for

The three outermost crates are each named for the one technology they may name, and `architecture.toml` is what makes that true rather than customary:

| Crate | May name |
| --- | --- |
| `adocpdf-asciidoc` | `asciidoc-parser` |
| `adocpdf-typst` | `typst`, `typst-layout`, `typst-pdf` |
| `adocpdf-host` | the operating system — it is the only crate naming `std::fs` |

There is one exception, and the model draws it rather than hiding it: `adocpdf-cli` may name Typst in tests, because the layout tests read the frames the engine produced.

## Keeping it true

Nothing checks this model against the code. `crates/` is the territory; when the two disagree, the code is right.

It needs updating when a crate gains or drops a dependency, when a port or adapter is added, when a module moves between crates, or when the order of steps in `RenderDocument::execute` changes.

## Not modelled

- **`xtask` and `fuzz`** — neither is a layer. `xtask` is tooling nothing may depend on, and `fuzz` sits outside the workspace and is never shipped.
- **Deployment** — nothing is deployed. The CLI is a binary a person runs.
- **`adocpdf-wasm`'s internals** — the crate is empty, and carries the `stub` tag to say so.
