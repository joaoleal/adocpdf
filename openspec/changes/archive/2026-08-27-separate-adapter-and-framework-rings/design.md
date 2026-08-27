## Context

See `proposal.md` — *Why*, and `docs/architecture/proposals.md`, which carries
the evidence and the cost of each of the three findings this change takes.

Everything below was established by reading the tree, not by building it: **no
`cargo` command was run while writing this plan**, because no toolchain is
installed where it was written. Every claim about what compiles is therefore a
task to verify, marked as such, rather than a fact. The claims about what the
code *says* were checked file by file.

Five facts about the current tree shape the decisions.

1. **`adocpdf-infra` has exactly four internal edges that cross the proposed
   split.** `renderer.rs` uses `emitter`, `fonts` and `world`; `emitter.rs` uses
   `markup`; `world.rs` uses `fonts` — all five stay inside `adocpdf-typst`. The
   two that cross are `parser.rs` → `clock::unix_timestamp` and `themes.rs` →
   `fonts::EmbeddedFonts`. Those two are D3 and D4, and they are the whole
   difficulty of the split.
2. **`themes.rs` names no Typst type, but depends on one.** It calls
   `EmbeddedFonts::provides_family` to reject a theme naming a family no
   embedded face supplies, and `EmbeddedFonts` wraps `typst::text::FontBook`.
3. **`clock::unix_timestamp` names no technology at all.** It is
   `days_from_civil(date) * 86_400` over a domain `Date`, and `clock.rs` holds
   its inverse, `date_from_unix_days`, used by `SystemClock` to turn a
   `SystemTime` into a `Date`. The parser adapter uses the forward direction to
   build `asciidoc_parser::ReferenceTime`.
4. **Half the integration tests span both technologies.** `tests/layout/mod.rs`
   imports `parser`, `emitter`, `fonts`, `world` and `themes`, and is included
   by `blocks.rs`, `headings.rs`, `inline_roles.rs`, `layout_helper.rs`,
   `list_presentation.rs`, `lists.rs`, `paragraph_fill.rs` and
   `paragraph_presentation.rs`. `hang_regressions.rs` and `support_inventory.rs`
   span both directly. That is ten of the twenty entries under `tests/`.
5. **The public surface of each adapter is already free of foreign types.**
   `AsciidocParser` exposes `new` and the `DocumentParser` impl; `TypstRenderer`
   exposes `new` and the `DocumentRenderer` impl. Nothing re-exports an
   `asciidoc_parser` or `typst` type, so no crate downstream of either can name
   one by accident.

## Goals / Non-Goals

**Goals:**

- Make the confinement of each external technology a rule the guard enforces,
  rather than one review enforces.
- Close the outbound half of the boundary the workspace already built, so no
  delivery crate names a domain result type.
- Change nothing a user can observe, and be able to prove it.
- Keep the two rules that are easy to break — content is never markup, paths are
  judged by where they resolve — intact and, where possible, easier to hold.

**Non-Goals (design-level; `proposal.md` has the scope non-goals):**

- Redesigning any port. The six ports keep their signatures; only the crate
  their implementations compile into changes.
- Improving a test while moving it. A moved test whose assertions changed is two
  changes wearing one commit.
- Making `adocpdf-wasm` build for the browser through the new crates. Whether
  the engine's dependency tree builds for WASM is still the open question
  `render-first-pdf` left open.

## Decisions

### D1 — The result crosses the boundary, and the presenter owns the wording

`main` converts with `from_domain_report` and passes `RenderReportDto` to a new
`presenter` module in `adocpdf-cli`. `report_success` moves there and takes the
DTO.

Two details decide whether this is a move or a rewrite:

- `RenderReport::is_complete()` is a domain method the DTO does not have. The
  presenter uses `skipped.is_empty()`, which is what `is_complete` returns.
- `SkippedConstruct::location` is a `SourceLocation`, whose `Display` writes
  `line {line}, column {column}`. The DTO carries the two numbers, not the
  formatted string, so the presenter formats them itself.

The second is a real duplication and it is the right one. A wire shape should
carry numbers; how a terminal spells a position is a presentation choice, and
the presenter is where presentation choices belong. The cost is that the two
spellings can drift apart. That is bounded — the domain's `Display` is used in
error messages, the presenter's in the skipped-construct report — and it is the
duplication a boundary buys, the same one `RenderReport` and `RenderReportDto`
already pay for.

**Verification is a characterisation test, because the existing tests are too
loose.** `crates/adocpdf-cli/tests/end_to_end.rs` asserts that stdout *contains*
`out.pdf` and `bytes`, and that stderr *contains* `skipped`. A reworded report
passes all of it. So the presenter gets a unit test asserting the exact lines
for a report with no skipped constructs and for one with two, written against
today's output before the conversion is made.

**What the CLI still names, deliberately.** `RenderDocument`, `Clock`, `Date`
and `DomainError`. A composition root constructs the use case and maps its
failures to exit codes; naming what it constructs is its job. The rule being
restored is narrower and worth stating precisely: **a result produced by the use
case must not reach the terminal without crossing the boundary built for it.**

*Alternative considered — delete the outbound half instead.* `proposals.md`
records it: drop `RenderReportDto` and `from_domain_report`, and say in
`adocpdf-shared` that the boundary is inbound-only. It is a coherent position
and it is cheaper. Rejected because `adocpdf-wasm` is the reason
`adocpdf-shared` exists, and a WASM binding that can accept a request but must
hand back a domain type would have to reintroduce exactly this code, minus the
tests that currently keep it honest. An unused outbound half is worse than an
honest absence; a *used* one is better than either.

### D2 — Four crates, and an allow-list that names only what each declares

| Crate | Ring | Workspace deps | External allow-list |
| --- | --- | --- | --- |
| `adocpdf-adapters` | Interface adapters | `adocpdf-domain`, `adocpdf-shared` | — |
| `adocpdf-asciidoc` | Frameworks | `adocpdf-core`, `adocpdf-domain`, `adocpdf-adapters` | `asciidoc-parser`, `proptest` (test-only) |
| `adocpdf-typst` | Frameworks | `adocpdf-core`, `adocpdf-domain` | `typst`, `typst-layout`, `typst-pdf`, `proptest` (test-only), `pdf-extract` (test-only) |
| `adocpdf-host` | Frameworks | `adocpdf-domain`, `adocpdf-adapters` | — |

Each list is what the manifest declares, not what the ring might plausibly want.
`architecture.toml` is a ceiling, and a ceiling set at the height of the room is
the only one that catches anything: the point of this change is that
`asciidoc-parser` appearing in `adocpdf-typst`'s manifest becomes a failed
build. Test-only crates are recorded in the same list with a comment saying so,
as `adocpdf-domain` already does for `proptest` — the guard reads dependencies,
dev-dependencies and build-dependencies into one list and has no `dev-external`
key.

`adocpdf-cli` and `adocpdf-wasm` have `adocpdf-infra` replaced by the four new
names in their `workspace` lists. `adocpdf-cli`'s manifest declares all four,
since `main` constructs an adapter from each; `adocpdf-wasm` still declares
nothing.

**No third-party crate is added, removed or re-versioned.** The versions in
`[workspace.dependencies]` — `typst`, `typst-layout`, `typst-pdf` 0.15.1
(Apache-2.0), `asciidoc-parser` 0.29.19 (MIT OR Apache-2.0), `proptest` 1.11.0
and `pdf-extract` 0.12.0 — are inherited unchanged by whichever crate now
declares them. `deny.toml` needs no edit and no new licence obligation is
created.

**Boundaries this touches, and what keeps them intact.**

- *Injection.* `markup.rs` and `emitter.rs` are one boundary — everything derived
  from a source reaches the output through `markup::string_literal` and nothing
  else — and they stay in one crate. After the split that boundary is stronger
  than it is today: only `adocpdf-typst` can produce Typst markup at all,
  because only it may name Typst.
- *Structure forgery.* `parser.rs`'s `refuse_input_that_could_forge_structure`
  imports both range predicates from `inline.rs`, deliberately, so the rule has
  one definition. Both files move together into `adocpdf-asciidoc`; splitting
  them would recreate the two-copies defect that change already found and fixed.
- *Sandbox.* `SandboxedPath` and the containment rule stay in `adocpdf-domain`.
  `FilesystemPathResolver` and `FilesystemSourceStore` — the only two modules
  that touch the filesystem — end up alone in `adocpdf-host`, which makes "no
  filesystem call outside the sandbox" a claim about one small crate.
- *Determinism.* The `Clock` implementations stay together in `adocpdf-host`, and
  D4 keeps the date conversion single-sourced so both engines still agree on
  what day it is.

### D3 — `themes.rs` goes behind the engine wall, and the reason is the font book

`BuiltInThemes` is policy — a fixed catalogue of themes compiled into the
binary — but `ThemeRepository::load` rejects a theme naming a family the
embedded faces do not provide, and the embedded faces are a Typst `FontBook`
(fact 2). Three placements were possible:

1. **`adocpdf-typst`.** The catalogue sits in a frameworks crate, which is one
   ring further out than its content deserves.
2. **`adocpdf-adapters`, with a dependency on `adocpdf-typst`.** This makes the
   adapter ring depend on the engine crate, which is the exact confinement the
   change exists to buy, and it points outward. Rejected outright.
3. **`adocpdf-adapters`, with a new `FontCatalogue` port owned by the domain and
   implemented in `adocpdf-typst`.** Canonically correct: the catalogue asks
   "does this family exist" through a port and never learns who answers.

Chosen: 1. Option 3 is better architecture and it is not this change. It adds a
port to the domain, an implementation, and wiring at both composition roots —
a design change inside a refactor whose whole claim is that nothing changes.
Doing it here would also make the split unverifiable: with behaviour, ports and
crate boundaries all moving at once, a failing test tells you nothing about
which of the three broke it.

Option 2's weaker cousin — leave the catalogue in the adapter ring and check
font availability later, in the renderer — was also rejected, and not on taste:
it moves the rejection of an invalid theme from load time to render time, which
is an observable behaviour change in a change that must not have one.

So the honest summary is that `adocpdf-adapters` is smaller than the name
suggests: it holds the DTO mapping and the calendar arithmetic of D4, and
nothing else. That is recorded in *Open Questions* as the thing to revisit,
because a ring holding one module is a ring worth re-examining once the port in
option 3 exists.

### D4 — The calendar arithmetic moves inward, not sideways

`parser.rs` needs `unix_timestamp` to build the parser's `ReferenceTime`, and it
lands in a different crate from `clock.rs` (fact 3). Three ways out:

1. **`adocpdf-asciidoc` depends on `adocpdf-host`.** A lateral edge between two
   frameworks crates. The guard would permit it because `architecture.toml`
   would say so, and every later reader would read that entry as licence for the
   next one. Rejected: the frameworks ring is a set of siblings wired together
   by a composition root, not a chain.
2. **Move it into `adocpdf-domain`, beside `Date`.** Defensible — it is
   arithmetic on a value object, not knowledge of a technology — but it widens
   the domain's public surface to serve two adapters, and "the domain grew a
   method because a refactor needed one" is a bad precedent.
3. **Move it into `adocpdf-adapters` as a `calendar` module.** Frameworks
   depending inward on interface adapters is the canonical direction, and
   converting between a representation the use case owns and one an external
   agency wants is precisely what the adapter ring is for.

Chosen: 3. `unix_timestamp` and `date_from_unix_days` move together — they are
inverses that agree by construction, and `AGENTS.md`'s own lesson is that a rule
enforced in two places will eventually be enforced in one. `adocpdf-host` and
`adocpdf-asciidoc` both depend on `adocpdf-adapters` for it, and `SystemClock`
keeps its `SystemTime` reading, which is the only part that is genuinely the
host's.

The unit tests in `clock.rs` split with the code: the two that assert the epoch
and midnight properties go to `adocpdf-adapters`, and the ones about
`SystemClock` and `FixedClock` stay with `adocpdf-host`.

### D5 — A test that parses *and* typesets belongs to the composition root

Ten of the twenty test entries reach both technologies (fact 4). After the split
no technology crate may host them, because hosting one means declaring the other
crate as a dev-dependency, and `architecture.toml` governs dev-dependencies
exactly as it governs dependencies.

They move to `crates/adocpdf-cli/tests/`, which already depends on all four new
crates in production and is the one place in the workspace where the assembled
system is legitimately visible. That is also what such a test *is*: parsing a
document and laying it out is an assembly of two frameworks, and assembling
frameworks is what a composition root does.

*Alternatives considered.* **Permit `adocpdf-typst` a dev-dependency on
`adocpdf-asciidoc`**, recorded with a reason as `proptest` is. Rejected: the
guard reads dev-dependencies and dependencies into one list, so permitting the
edge for a test permits it for production, and the entry that says "test-only"
is a comment the guard cannot read. Buying back convenience with the exact
looseness this change is removing would be self-defeating. **A dedicated
`crates/adocpdf-e2e` crate holding nothing but tests.** Cleaner separation from
the binary's own end-to-end suite, at the price of a crate that exists to hold
test files, a ninth `architecture.toml` entry, and a new thing to explain in
`AGENTS.md`. The composition root already exists and already does this.

The cost, stated: `crates/adocpdf-cli/tests/` roughly triples in size, and tests
about how a paragraph is laid out end up two rings away from the emitter they
exercise. Coverage is unaffected — `cargo llvm-cov --workspace` counts the
workspace, not the crate — and the tests themselves are moved verbatim.

*Found when applying.* The cost is one step larger than written above.
`tests/layout/mod.rs` does not merely use the two engine crates, it names
`typst` itself — it calls `typst::compile` and walks `typst::layout::Frame` to
read what was laid out. So `adocpdf-cli` gains `typst` and `typst-layout` as
third-party dev-dependencies, and `architecture.toml` must permit them there.
It also gains `adocpdf-core`, which it had always been permitted but had never
declared.

This is unavoidable while assertions stay verbatim: a test that inspects engine
frames has to name the engine's geometry types. It is confined to
`[dev-dependencies]` — nothing under `crates/adocpdf-cli/src/` names Typst — but
the guard reads one list, so the permission is real and a future production use
would not be caught. The claim that survives is narrower than the one in
`AGENTS.md` today: only the two engine crates name a technology *in production
code*. Task 8.1 must write that sentence, not the wider one.

*Also found: there are eleven such tests, not ten.* Fact 4 miscounted.
`tests/emission_mechanism.rs` builds its input through `AsciidocParser` and then
compiles the result with Typst, so it reaches both technologies exactly as the
other ten do. It was invisible to the count because it reads as an emitter test,
which is what it is named for.

It surfaced the moment `parser.rs` left: the file could no longer compile where
it stood. The only alternative to moving it is a dev-dependency from the Typst
crate on the AsciiDoc crate — the lateral edge this decision rejects, and one
the guard would have to permit for production too. So it moved to
`crates/adocpdf-cli/tests/` with the other ten, one `use` line changed.

The lesson is about the counting, not the file: "tests that reach both
technologies" cannot be enumerated by reading test names. The compiler
enumerated it correctly, and only after the crate boundary existed to make the
question answerable.

### D6 — Rejected: moving the boundary DTOs into the domain

Canonical Clean Architecture puts request and response models at the use-case
boundary, which would mean `RenderRequestDto` and `RenderReportDto` living in
`adocpdf-domain` and the mapping disappearing.

Rejected, and the reason is recorded in the project context rather than invented
here: the DTOs sit in `adocpdf-shared` so a delivery crate can describe a request
without linking the domain, which is what keeps the WASM surface small. That is
a real constraint and the current placement serves it. `RenderRequest` in the
domain is already the input boundary model; `adocpdf-shared` holds the wire
shape outside it. Both exist for a reason, and D1 makes the outbound half of that
arrangement actually work rather than removing it.

### D7 — Rejected: an output port for the use case

Martin's output boundary exists so a use case does not depend on its presenter.
`RenderDocument::execute` returns a `Result` instead, and in Rust that inverts
nothing that needs inverting: the caller already chooses what to do with the
value. An output-port trait would add a `dyn` dispatch hop to buy a decoupling
the return type already provides, and it would put the presenter's shape into
the domain's vocabulary — the opposite of what D1 achieves by converting at the
edge.

Recording it matters because D1 is the change that would otherwise invite it: a
reader who sees "delivery must not name a domain result" may reach for an output
port next. The answer is that the DTO *is* the output boundary here, and it is
crossed by a function call rather than by a trait.

*Also rejected: splitting `adocpdf-core` from `adocpdf-domain`.* They are
already the entity and use-case rings, correctly separated — `adocpdf-core`
holds the model and depends on nothing, `adocpdf-domain` holds the ports and the
use case. This is recorded here only because it was written down elsewhere while
the split was still a proposal; the `rings` view now shows it, so it no longer
needs prose.

### D8 — The extraction order, and why the last step is a rename

The workspace must build after every task, so the split runs inward-out:
`adocpdf-adapters` first (nothing depends on it that does not already exist),
then `adocpdf-host`, then the cross-technology tests, then `adocpdf-asciidoc`.
At each step `adocpdf-infra` keeps everything not yet moved and gains a
dependency only on `adocpdf-adapters` — inward, and transitional.

*Corrected when applying: "permanent" was wrong, and D2's table is right.*
`parser.rs` was the sole consumer of `adocpdf-adapters` inside `adocpdf-infra`,
so the dependency left with it in task 5.1. The same is true of
`adocpdf-shared`, which `dto.rs` was the only user of and which has been
unused since task 2.1 — that one was already unused at HEAD.

So `adocpdf-typst` ends with `adocpdf-core` and `adocpdf-domain` and nothing
else, exactly as D2 states. Both stale entries come out of the manifest and the
allow-list at task 6.1, which rewrites that entry anyway. `cargo-machete` at
9.2 is what would catch it if 6.1 forgot: after a split, a leftover dependency
is the evidence that a module did not really move.

After `adocpdf-asciidoc` leaves, what remains in `adocpdf-infra` is exactly the
Typst crate's contents. The last step is therefore a **rename** of the directory
and the package — `git mv crates/adocpdf-infra crates/adocpdf-typst` plus the
`name` field — not a fifth extraction into an empty crate. Two reasons: the
guard requires the directory name and the package name to agree, and a rename
keeps `git log --follow` working across six files and seven font assets that
would otherwise look deleted and re-added.

The one thing that does not survive this ordering is `fuzz/`. It sits outside
the workspace, the gate never builds it, and its manifest names
`../crates/adocpdf-infra` by path — so the moment `adocpdf-asciidoc` leaves, the
fuzz crate is broken and nothing in the gate says so. It gets an explicit task
with an explicit build.

### D9 — The ring goes in `description`, not in the crate name

Each of the seven manifests states its ring in `description`, in the shape
`adocpdf-core` already uses ("Innermost ring: the document and theme model. No
dependencies."). Renaming crates to `adocpdf-entities`, `adocpdf-usecases` and
so on was considered and is a non-goal: the names describe position in this
workspace, which is useful, and a rename touches every import for a second time
in one change.

`description` is the right field because `cargo metadata` and `cargo doc` both
surface it, so the claim is visible where a reader already looks. It can still
go stale — nothing checks a description against reality — and that is the honest
limit of this part. It is documentation with the best available placement, not a
check.

### D10 — What this still does not enforce

Written down because the change's whole argument is about what a guard can see.

- **Dev-dependencies are indistinguishable from dependencies.** The guard reads
  both into one list. Permissions written for a test crate are permissions for
  production code, which is why D5 refuses the lateral edge rather than
  annotating it.
- **`fuzz/` remains structurally invisible.** It is outside the workspace by
  design, for the licensing reason in `deny.toml`, and it will depend on
  `adocpdf-asciidoc` and `adocpdf-typst` with nothing checking that it may. The
  comment in `architecture.toml` that says so must be updated to name the new
  crates rather than `adocpdf-infra`.
- **A description that lies is not caught.** See D9.
- **Confinement is checked at the manifest, not at the `use` line.** A crate that
  is permitted `typst` may name it anywhere within itself. That is the same
  granularity as today, one ring smaller — which is the entire improvement being
  bought, stated so nobody mistakes it for something finer.

## Risks / Trade-offs

- **The largest mechanical diff in this repository's history, touching files no
  test covers.** → Sequenced so the gate runs green after every task, and every
  file move is a `git mv` with no edit beyond the crate name in an import. The
  documentation and workflow files are the exposed part: `mutants.yml`'s
  enforced-file list and `_typos.toml`'s exclusion path fail *open* if they are
  missed — mutation testing enforces nothing and the spell-checker starts
  reading font binaries — so each gets its own task with its own verification.
- **A moved test could be quietly weakened.** → Tests move verbatim; the only
  permitted edit is the crate name in a `use` line. A diff that shows an
  assertion change in a move task is a defect in the task.
- **The CLI report could change wording without anything failing.** → D1's
  characterisation test is written *before* the conversion, against today's
  output.
- **`adocpdf-adapters` ends up holding two small modules.** → Stated rather than
  disguised. The ring is real even when thin, and D3 records what would fill it.
- **The split could turn out to be impossible as drawn**, if some coupling not
  visible in the imports appears once the compiler is involved. → The two
  crossings that exist were found by reading every `crate::` reference in
  `src/`, and both are handled by D3 and D4. If a third appears, it is a finding
  to record in this design before it is worked around.
- **`adocpdf-adapters` may or may not build for `wasm32-unknown-unknown`.** It
  depends only on `adocpdf-domain` and `adocpdf-shared`, so it should — and if
  it does, adding it to the gate's `WASM_CLEAN_CRATES` is free evidence that the
  adapter ring really is engine-free. **Unverified**; it is a task, and a failure
  there is a finding worth writing down rather than a step to skip.
- **Nothing forces the four crates to stay separated once they exist.** → That
  is what `architecture.toml` now says, and the guard fails the build on it.
  That is the change.

## Migration Plan

Not applicable in the deployment sense: nothing is released, the binary keeps
its name, and no crate is published. Within the workspace the sequence is the
one in D8, and `tasks.md` holds it. Each step leaves the workspace building and
`scripts/ci/gate.sh` green; the final task runs the gate once more against the
finished shape, and the fuzz target is built by hand because the gate will not
do it.

## Open Questions

- **Whether `adocpdf-adapters` should acquire the `FontCatalogue` port from D3**,
  bringing the theme catalogue back into the interface-adapter ring where it
  belongs. It is a separate change with its own behaviour surface, and it is the
  first thing to consider once this one lands.
- **Whether the cross-technology tests are better served by a dedicated test
  crate** than by the composition root (D5). The answer will be obvious after
  living with `crates/adocpdf-cli/tests/` at three times its current size, and
  moving them again later is cheap in a way that getting the crate boundaries
  wrong is not.
- **Whether `adocpdf-shared` should be described as the interface-adapter ring
  or as something outside the rings entirely.** It holds wire shapes with no
  behaviour, which Martin does not obviously have a ring for; `proposed.c4`
  colours it as interface adapters and D9 will follow that unless a better
  answer appears while writing the seven descriptions.
