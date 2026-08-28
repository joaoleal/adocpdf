## Why

`docs/architecture/proposals.md` records three findings against this workspace,
measured against Clean Architecture as Martin describes it. The first is a
defect; the other two are trade-offs. This change takes all three, because the
first is small and the second is what makes the third worth writing down.

**The outbound boundary is bypassed.** `adocpdf-shared` defines
`RenderReportDto` and `adocpdf-infra::dto::from_domain_report` produces it.
Nothing calls that function outside its own tests. `adocpdf-cli::main` hands the
domain `RenderReport` straight to `report_success`, which formats it for the
terminal. The inbound half of the same boundary *is* used — a request cannot
reach the use case without crossing it — so the boundary is asymmetric: a
request must be converted, a result need not be. The rule that keeps delivery
replaceable is that a delivery crate names no domain type, and today the CLI
names one on the way out. It also leaves production code with no production
caller, kept alive by its own tests, which reads as available when it is not.

**One crate holds two rings.** `adocpdf-infra` carries the DTO mapping, six port
implementations, the Typst emitter, the markup escaping, the engine world, the
embedded fonts and the AsciiDoc inline recovery. In Martin's terms that is the
interface-adapter ring and the frameworks-and-drivers ring in one place, and
`docs/architecture/views.c4` already colours the crate for two rings because it
could not honestly colour it for one.

The cost is not aesthetic. `architecture.toml` states permissions **per crate**,
so it currently permits `typst` and `asciidoc-parser` anywhere inside
`adocpdf-infra` — including in `dto.rs`, which must never name either. The
confinement that `AGENTS.md` states as a rule ("the only layer naming an
external technology", each technology in its own place) is held today by
discipline and review. The guard cannot see it. Splitting the crate is what
turns that convention into something the build checks.

**The rings are named only in a diagram.** Crate names describe position in this
workspace — `core`, `domain`, `shared`, `infra` — rather than role in the
pattern, and the mapping lives in prose in `AGENTS.md` and in a LikeC4 view.
`adocpdf-core`'s `description` field already does the job for one crate
("Innermost ring: the document and theme model"); the others do not.

This is not urgent, and the proposal it comes from says so: nothing is broken by
the second finding, the guard is merely coarser than the intent it protects. It
is proposed now because the cost of the split only grows with the number of
modules on either side of the line, and because the first finding is a real
defect sitting in the same code.

## What Changes

- **The CLI converts its result and formats the DTO.** `main` calls
  `from_domain_report`, and a new `presenter` module in `adocpdf-cli` formats
  `RenderReportDto` for the terminal. The CLI keeps naming the use case, the
  `Clock` port, `Date` and `DomainError` — a composition root must name what it
  constructs and what it maps to an exit code. What it stops naming is a domain
  **result** travelling outward past a boundary built to carry it.
- **`adocpdf-infra` becomes four crates**, and the crate name disappears:

  | Crate | Ring | Holds |
  | --- | --- | --- |
  | `adocpdf-adapters` | Interface adapters | `dto.rs`, and the calendar arithmetic both engines need |
  | `adocpdf-asciidoc` | Frameworks | `parser.rs`, `inline.rs`. The only crate permitted `asciidoc-parser` |
  | `adocpdf-typst` | Frameworks | `renderer.rs`, `emitter.rs`, `markup.rs`, `world.rs`, `fonts.rs`, `themes.rs`, `assets/`. The only crate permitted `typst*` |
  | `adocpdf-host` | Frameworks | `clock.rs`, `source_store.rs`, `path_resolver.rs` — the technology is the OS |

  `architecture.toml` gains an entry per crate, and each external allow-list
  names only what that crate declares. `asciidoc-parser` in the Typst crate, or
  `typst` in the mapping crate, then fails the build rather than a review.
- **Two modules the proposal's table does not name are placed, with a reason.**
  `themes.rs` goes to `adocpdf-typst` because its rejection of a theme naming an
  unavailable font family is a fact about a Typst `FontBook`; the calendar
  arithmetic in `clock.rs` goes to `adocpdf-adapters` because `parser.rs` needs
  it and one frameworks crate must not reach sideways into another. Both are
  design decisions, D3 and D4, not incidental placements.
- **Every test moves with the code it exercises, and the tests that span two
  technologies move to the composition root.** Roughly half of
  `crates/adocpdf-infra/tests/` parses a document *and* lays it out, so after the
  split no single technology crate may host them. They go to
  `crates/adocpdf-cli/tests/`, which is the one place already permitted to see
  both. Design D5 records the alternative — permitting a dev-dependency between
  the two engine crates — and why it is refused.
- **Each crate's `Cargo.toml` `description` states its ring**, in the shape
  `adocpdf-core` already uses. `cargo metadata` then reports the architecture,
  and `cargo doc` shows it beside every crate.
- **The documentation that carries the layer table is updated**: `AGENTS.md`,
  `CONTRIBUTING.md`, `openspec/config.yaml`, `README.md`, and the LikeC4 model
  under `docs/architecture/`. The three proposals this change implements are
  removed from `proposals.md` and `proposed.c4`, because a proposal that has
  been taken is no longer a proposal.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. Every requirement in `openspec/specs/` holds exactly as written before and
after: the same source produces the same PDF bytes, the binary prints the same
lines on the same streams, and every failure keeps its exit code. `.openspec.yaml`
therefore sets `skip_specs: true`, following `quality-gates` and
`check-commit-messages-in-the-gate`.

The one place to be careful is the CLI's report, since the code that formats it
is rewritten against a different type. The requirement it serves — that an
omission is never silent — is unchanged, and the change is held to
byte-identical output by a characterisation test rather than by assertion; see
design D1. Inventing a capability to describe a crate split would put a
spec-shaped claim on something no user can observe.

## Non-goals

- **Renaming `core`, `domain` or `shared`.** The names describe position, the
  descriptions will describe role, and renaming three more crates would double a
  refactor that is already wide for no gain the description does not give.
- **Changing any behaviour.** No refusal is widened, no error message is
  reworded, no theme is added, nothing renders differently. Any diff that
  changes output is a defect in this change, not a scope decision.
- **Moving the boundary DTOs into the domain.** Considered and rejected in
  design D6; `adocpdf-shared`'s placement serves a constraint that still holds.
- **Adding an output port to the use case.** Considered and rejected in design
  D7. `RenderDocument::execute` returning a `Result` already gives the caller
  the choice an output boundary exists to give.
- **Splitting `adocpdf-core` from `adocpdf-domain`.** They are already the entity
  and use-case rings, correctly separated.
- **Making the WASM crate do anything.** It stays a stub. Its
  `architecture.toml` entry is updated to name the new crates, which is
  bookkeeping, not a feature.
- **Adding a dependency.** No third-party crate is added or removed by this
  change, so `deny.toml`, `Cargo.lock`'s external contents and `.cargo/audit.toml`
  need no licence or advisory work.

## Ordering

Nothing depends on this and it depends on nothing. It is deliberately sequenced
so the workspace builds and the suite passes after every task: the presenter
lands first while `adocpdf-infra` is still one crate, then the adapter ring is
extracted, then the host adapters, then the tests that span two technologies
move to the composition root, then the AsciiDoc crate leaves, and what remains
is renamed to `adocpdf-typst`. See design D8 for why the last step is a rename
rather than a fifth extraction.

## Impact

**Layers: all of them, and the file that defines them.** No layer gains or loses
a rule; four crates appear where one was, and `architecture.toml` grows from six
entries to nine.

**Blast radius — stated plainly, because it is the main cost.** Every import of
`adocpdf_infra` changes. That is:

- `crates/adocpdf-cli/src/main.rs`, which names six of its modules.
- All twenty entries under `crates/adocpdf-infra/tests/`, including
  `parser_refusal.rs`, `known_crashes.rs`, `hang_regressions.rs`,
  `termination_sweeps.rs` and the `layout/` helper that seven other test files
  include. Ten of them must also change crate, not merely their import path.
- `fuzz/Cargo.toml` and `fuzz/fuzz_targets/parse_plan_emit.rs`, which reach the
  parse → plan → emit path directly. **`fuzz/` is outside the workspace and the
  gate never builds it**, so a stale path there survives a green gate and is only
  discovered by the weekly nightly workflow. It gets its own task.
- `.github/workflows/mutants.yml`, whose enforced-file list names
  `crates/adocpdf-infra/src/markup.rs` and `.../inline.rs` by path. A path that
  no longer exists makes mutation testing enforce nothing, silently.
- `_typos.toml` (the font-asset exclusion path), `LICENSING.md`, `README.md`,
  `SECURITY.md`, `.cargo/audit.toml`, `docs/asciidoc-support.md`,
  `fuzz/known-crashes.toml` and `scripts/ci/known-crashes.sh`, each of which
  names a path or a crate in prose or in data.
- `AGENTS.md`, `CONTRIBUTING.md` and `openspec/config.yaml`, which carry the
  layer table three times over.

**The workspace root manifest needs no edit for the new crates.** `members` is
the glob `crates/*`, so a new directory joins the workspace by existing; the
guard is what refuses an ungoverned crate, not the manifest. `Cargo.lock` gains
three package entries and loses one name. This was checked rather than assumed,
because a refactor plan that says "update the workspace members" and finds
nothing to update has misread the thing it is about to change.

**Nothing is deleted that is not moved.** Every module, test and asset that
exists today exists afterwards, in a different crate. `git mv` keeps the history
attached; a copy-and-delete would not.

**The gate.** Coverage is workspace-wide, so moving a test between crates cannot
move the number. `cargo-machete` becomes a useful check on this change
specifically: after the split, a crate declaring a dependency it does not use
fails the unused-dependencies job, which is exactly the evidence that each
technology really did end up in one crate.
