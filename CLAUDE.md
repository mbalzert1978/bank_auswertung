# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Architektur-Umbau nach [ADR-0007](docs/adr/0007-modularer-monolith-feature-slices.md)
> — strukturell abgeschlossen (Phasen A–F).** Das Repo ist von „ein geteiltes
> `domain`-Crate + Adapter" auf einen **modularen Monolithen aus Feature-Slices**
> umgestellt: pro Slice eine Triade `<slice>_domain`/`_adapter`/`_specs`, dazu ein
> **dünner** interface-only `shared_kernel` und das Top-Validator-Crate
> `architecture` (Dependency-Richtungs-Test über `cargo_metadata`). Das geteilte
> `domain`-Crate ist aufgelöst; jeder Slice hat eigene Value Objects und gibt nach
> außen nur DTOs heraus. Der „Current state" unten beschreibt diesen Stand;
> **maßgeblich ist jetzt [ADR-0008](docs/adr/0008-event-getriebene-pipeline-und-outbox.md)**
> (event-getriebene Pipeline aus Slices mit Outbox), das ADR-0007 in Teilen
> supersedet/verfeinert.

## What this is

**Bank-Auswertung** — local, maintainable Rust software that reads bank-account
transactions and surfaces the main cost drivers and realistic savings potential.
**Pure analysis**: it never triggers transactions and never changes account data.

The project is **docs-first**: the architecture is specified ahead of
implementation. The design is the point — read the docs before implementing:

- [README.md](README.md) — source of truth for scope and architecture.
- [CONTEXT.md](CONTEXT.md) — domain glossary (the ubiquitous language).
- [docs/adr/](docs/adr/) — the seven architecture decisions, each with rationale
  (ADR-0007 verfeinert/supersedet die Architektur-Aussagen von 0001–0003).
- [docs/prd/0001-bank-auswertung.md](docs/prd/0001-bank-auswertung.md) — PRD
  (mirrors the README; adds user stories and the module breakdown).

**The ADR-0007 slice refactor is structurally complete (Phases A–F).** The repo
is a **Cargo workspace** (edition 2024; every member inherits the strict
`[workspace.lints.clippy]` "Don't Panic" set via `[lints] workspace = true`) of
**ten crates under `crates/`** plus the top-level validator crate
[`architecture`](architecture):

- a **thin, interface-only** [`shared_kernel`](crates/shared_kernel) — messaging
  traits only, unwired, no deps and no value objects;
- three **feature-slice triads** `<slice>_domain`/`_adapter`/`_specs`:
  [`eigene_konten`](crates/eigene_konten_adapter),
  [`obfuskierung`](crates/obfuskierung_adapter), and
  [`csv_camt_v8`](crates/csv_camt_v8_adapter);
- [`architecture`](architecture/tests/dependency_direction.rs) — a test-only crate
  (outside `crates/`) that asserts the dependency direction over `cargo_metadata`.

An earlier Python/`uv` scaffold **and** the earlier shared `domain` crate were both
deliberately removed ([ADR-0003](docs/adr/0003-rust-als-sprache.md),
[ADR-0007](docs/adr/0007-modularer-monolith-feature-slices.md)); ignore any
lingering Python or `crates/domain` references.

**The hexagonal cut is enforced three ways:** (1) at the crate level — a
`<slice>_domain` crate is **dependency-free** (not even `serde`/`thiserror`;
errors are hand-written `Display` + `std::error::Error`, the adapter maps them),
and a slice reaches a foreign slice only through its public `_adapter`, never its
`_domain`; (2) by the `architecture` test (`cargo test -p architecture`), which
reads the resolved graph and fails on any violated inward edge (domain not blind,
external dep outside `io`, foreign-`_domain` coupling, specs not black-box); (3) by
the `enforce-dependency-direction.py` PreToolUse hook, which blocks a
non-sanctioned external `[dependencies]` line at edit time in every crate except
the single deferred `io` crate and `architecture`.

Per slice (value objects live in the owning slice; only DTOs cross the boundary):

- **`eigene_konten`** — `_domain` owns `IbanEntity` (checksum-validated, mod-97)
  and `RolleEntity`; `_adapter` exposes the `Konfigquelle` seam, the DTOs `Iban`
  (inbound, **no `Serialize`**), `Rolle` and `Zuordnung`, the `Kontenregister`,
  and the Fluent `KontenTestApi`. Single source for `Gegenkonto::Eigen|Fremd`.
- **`obfuskierung`** — `_domain` owns the First-Seen mapping aggregate
  (`KONTO_x`/`PERSON_x`, run-stable) and the `ZuordnungsSpeicher` port it
  requires; `_adapter` exposes `obfuskiere_konto/person/text`, the `Iban` inbound
  DTO (**no `Serialize`**), the `KontoId`/`Pseudonym`/`ObfuskierterText` outbound
  DTOs, and a Fluent TestApi with an in-memory fake store. Persistence is deferred
  ([ADR-0006](docs/adr/0006-pseudonym-stabilitaet.md)).
- **`csv_camt_v8`** — `_domain` owns `BuchungAggregat` + its value-object entities
  and the **field parsers** (`BetragEntity::aus_deutschem_betrag`,
  `BuchungsartEntity::aus_buchungstext`, `RichtungEntity::aus_vorzeichen`, each
  unit-tested in the domain module); `_adapter` fulfils the `Umsatzquelle` inbound
  port, defines the `Zeilenquelle` seam (io delivers already-parsed `Umsatzzeile`
  DTOs — byte-level CSV parsing is deferred to `io`), bridges to
  `eigene_konten_adapter` and `obfuskierung_adapter` (Eigen/Fremd +
  Auftragskonto validation), and **obfuscates while mapping** so raw IBAN/BIC never
  reach a `Buchung`. `Buchung` is the outbound DTO; the Fluent `CsvImportTestApi`
  drives the real adapter behind fakes.

**Privacy invariant (compiler-enforced):** the raw `Iban` DTOs derive **no
`Serialize`**, proven per slice by a `static_assertions` `assert_not_impl_any!` —
embedding a structural identifier in a serializable DTO is a compile error, not a
review miss. `Bic` is deferred until a format needs it.

Every crate is green (`cargo build`/`test`/`clippy --all-targets`, `cargo fmt
--check`). Deferred (target picture, not yet built): the single `io` crate
(`csv`/`toml`/`fs`/`sqlx` + `Speicher`), the `presentation` crate, the messaging
layer, the analysis/categorizer slices (incl. `Fingerabdruck`, `KategorieAnfrage`),
`ObfuskierterText` scrubbing (only trims/rejects-empty today), and the persistent
pseudonym mapping. Next functional work: **the analysis core** (recurrence + fees +
cashflow + Pareto — valuable without the LLM). No binary yet.

## Language convention

The project language is **German**, so all docs and tests are written in **German**. Everything else, including code, is written in **English**.

## Commands

Standard Cargo (Python `uv` tooling is gone). Run from the repo root:

```bash
cargo build                 # compile
cargo test                  # run all tests
cargo test <name>           # run tests whose name matches <name> (single test)
cargo clippy --all-targets  # lint
cargo fmt                   # format
```

There is no binary yet (library crate, no `src/main.rs`), so `cargo run` does
nothing useful until a binary target exists.

Git worktrees for this repo go under `.claude/worktrees/` (local-only,
gitignored — not a project directory).

## Architecture — the big picture

Hexagonal (Ports & Adapters), realized as a **modular monolith of feature-slice
triads** in a Cargo workspace ([ADR-0007](docs/adr/0007-modularer-monolith-feature-slices.md)).
The hexagonal cut is enforced **at the crate level** — a `<slice>_domain` crate is
dependency-free and depends on no I/O crate (no `reqwest`, no `rusqlite`), so it
*cannot* do I/O; the direction is guarded by the `architecture` test and the
PreToolUse hook
([ADR-0001](docs/adr/0001-hexagonale-architektur.md),
[ADR-0003](docs/adr/0003-rust-als-sprache.md)).

**The domain is the analysis; the LLM is only an oracle**
([ADR-0004](docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)).
Reading, obfuscating, and presenting are adapter jobs. The pure domain does:
recurring-outflow detection (incl. yearly costs and price increases), transfer
(`Umbuchung`) netting, fee classification, cashflow per account and month,
Pareto ranking, savings candidates, and the category register with learned
corrections. **Litmus test**: remove the LLM entirely and the program is still
valuable — only automatic labels for unknown merchants are missing.

Four ports:

- **`Umsatzquelle`** (inbound) — CSV-CAMT-v8 adapter first. **Obfuscates while
  mapping** so the domain only ever receives obfuscated value objects.
- **`Kategorisierer`** (outbound) — the LLM behind a port. Chat-payload adapter
  first (export unknown fingerprints → chat → import), REST/API adapter later.
- **`Speicher`** (outbound) — SQLite, stores **DTOs** (not domain objects).
- **`Bericht`** (outbound) — HTML first; shows the domain's analysis, not "what
  the LLM said".

Adding another input format (XML-camt, MT940) or output/categorizer is a new
adapter behind an existing port — **no domain change**. These are deliberately
out of scope for now (YAGNI).

## Non-negotiable invariants

These are the reason the project exists; do not weaken them.

**Privacy / obfuscation** ([ADR-0003](docs/adr/0003-rust-als-sprache.md),
[ADR-0001](docs/adr/0001-hexagonale-architektur.md)):

- Structural identifiers (own IBAN/account no., counter-IBAN, BIC) **never**
  leave the machine, and this is **compiler-enforced**: raw newtypes `Iban` /
  `Bic` live only in the inbound adapter, derive **no `Serialize`**, and are
  embeddable in **no** DTO — a leak is a compile error, not a review miss.
  (`serde` is allowed; the guarantee rests on the missing `Serialize`, not on
  banning `serde`.)
- The domain receives **no** person- or account-relatable identifier — only
  obfuscated value objects. The **`Gläubiger-ID`** (public company key) is a
  deliberate exception, kept as an analysis anchor.
- Free-text (Empfänger/Verwendungszweck) must pass through the single value
  object **`ObfuskierterText`**, whose constructor best-effort scrubs embedded
  identifiers. This scrubbing is **heuristic, not provably complete** — state
  that honestly; do not claim anonymization.
- **Fail-safe** "Firma vs. Privatperson": when unsure, treat as `Person` and
  pseudonymize. The costlier error is leaking a real name.

**DTO boundary / Anti-Corruption-Layer**
([ADR-0002](docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)):

- The domain's only public surface is DTOs. **Each port speaks its own narrow
  DTO**, never a shared god-DTO. The `Kategorisierer` port speaks
  `KategorieAnfrage` (fingerprint + `ObfuskierterText`) outbound and
  `KategorieAntwort` (fingerprint → category) inbound — the answer keys on the
  **fingerprint, never the text**, so re-identification is structurally
  impossible.

**Fingerprint is domain knowledge**
([ADR-0004](docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md),
[ADR-0006](docs/adr/0006-pseudonym-stabilitaet.md)): `Buchung → Fingerabdruck`
is a pure domain function (incl. payment-intermediary detection + merchant
token), **not** an adapter concern. The LLM is asked only about fingerprints the
domain does not yet know.

**Two separate local stores** (never mix — mixing is a leak path)
([ADR-0006](docs/adr/0006-pseudonym-stabilitaet.md)): the `Speicher` port holds
only obfuscated DTOs; a separate **plaintext↔pseudonym mapping store** never
leaves the machine and is **persistent** so `PERSON_x` tokens stay stable across
imports (an unstable pseudonym would break the fingerprint of person-to-person
standing orders).

## Domain model conventions ([ADR-0005](docs/adr/0005-domaenenmodell-value-objects-und-summentypen.md))

- **Value objects, not primitives** (_parse, don't validate_): `Betrag`,
  `KontoId`, `Pseudonym`, `Fingerabdruck`, `GläubigerId`, `ObfuskierterText`.
  The non-empty string newtypes share one validating core, `NonEmptyString`
  (trim + reject-empty in one place), and are built from it via `From`.
  Validate once at the edge; the type then carries the proof. No raw
  `String`/`f64` crosses the DTO boundary into the domain.
- **Sum types with exhaustive `match`**, not a flag plus optional fields:
  `Richtung::Eingang|Ausgang`, `Gegenkonto::Eigen(Rolle)|Fremd`,
  `Empfänger::Firma|Person`, `Buchungsart`. Illegal combinations must be
  **unconstructible**, not "checked".
- `Betrag` is **unsigned** with a currency; the sign lives only in `Richtung`
  (a negative `Eingang` is unconstructible). The domain does **not** sum mixed
  currencies — the inbound adapter guarantees a single currency.
- Style is **declarative-functional**: pure transformations over immutable
  values, collection pipelines (`map`/`filter`/`fold`) over manual loops,
  `match` over `if/else` cascades. Side effects live at the edges (adapters).

## Testing ([ADR-0002](docs/adr/0002-dto-grenze-und-anti-corruption-layer.md), [ADR-0005](docs/adr/0005-domaenenmodell-value-objects-und-summentypen.md))

- **Black-box at the DTO boundary**: test the domain against its public DTO
  contract, not its internals — the inside stays freely refactorable.
- **TDD outside-in**, organized as **vertical slices** (one per build-order
  step). The load-bearing invariant — the projection `Buchung → KategorieAnfrage`
  **carries no structural identifier** — is realized in step 1 as a **compile-time**
  `static_assertions` check that `Iban`/`Bic` derive no `Serialize` (strictly
  stronger than a runtime assertion); the projection's content and constructor
  validation are covered by runtime black-box tests.
- Constructor validation is tested too: an illegal value yields **no object**.

## Build order (tracer-bullet, vertical slices)

Build in this order; each step leaves something runnable/valuable:

1. **Cargo workspace + domain skeleton** — `Buchung` from value objects + the
   narrow `KategorieAnfrage` outbound type, with the no-structural-identifier
   test. _(Replaces the old Python hello-world.)_ — **done** (see Current state).
2. **CSV-CAMT-v8 adapter** against a real sample file → `Buchung`, obfuscating
   while mapping. **Blocked** on a real sample file and the Eigene-Konten-Config.
3. **Analysis core** — recurrence + fees + cashflow + Pareto (valuable without
   the LLM).
4. **`Speicher` (SQLite)** for DTOs + category register. Needs the persistent
   pseudonym mapping first.
5. **`Kategorisierer` chat-payload adapter** (export → chat → import). Needs the
   `ObfuskierterText` scrubbing rules first.
6. **HTML `Bericht`**.
7. **Iteration** — review together, learn corrections, name savings.

Prerequisites that must exist before their step (see README "Offen" / PRD): a
real CSV-CAMT-v8 sample file, the **Eigene-Konten-Config** (per own account:
IBAN + role Giro/Haushalt/Spar/Vorlauf — the single source for
`Gegenkonto::Eigen|Fremd` and for the analysis's role interpretation), the
**persistent pseudonym mapping**, and the **scrubbing rules** for
`ObfuskierterText`.
