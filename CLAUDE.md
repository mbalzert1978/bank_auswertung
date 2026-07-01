# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Bank-Auswertung** — local, maintainable Rust software that reads bank-account
transactions and surfaces the main cost drivers and realistic savings potential.
**Pure analysis**: it never triggers transactions and never changes account data.

The project is **docs-first**: the architecture is fully specified before code
exists. The design is the point — read the docs before implementing:

- [README.md](README.md) — source of truth for scope and architecture.
- [CONTEXT.md](CONTEXT.md) — domain glossary (the ubiquitous language).
- [docs/adr/](docs/adr/) — the six architecture decisions, each with rationale.
- [docs/prd/0001-bank-auswertung.md](docs/prd/0001-bank-auswertung.md) — PRD
  (mirrors the README; adds user stories and the module breakdown).

## Current state

A **skeleton only**. `Cargo.toml` is a single library crate (`bank_auswertung`,
edition 2024) with no dependencies; [src/lib.rs](src/lib.rs) is an empty
doc-comment stub — **no domain code, no types, no tests yet**. An earlier
Python/`uv` scaffold was deliberately replaced by this Rust crate
([ADR-0003](docs/adr/0003-rust-als-sprache.md)); ignore any lingering Python
references.

Building starts on Markus' explicit **"Go"** for step 1 of the build order below.

## Language convention

The project language is **German**. Docs, the glossary, code identifiers, and
commit messages are German. Keep domain identifiers in German exactly as
[CONTEXT.md](CONTEXT.md) defines them (`Buchung`, `Betrag`, `Empfänger`,
`Verwendungszweck`, `Buchungstext`, `Gläubiger-ID`, `Fingerabdruck`,
`Umbuchung`, `Gebühr`, …) — do not anglicize domain terms, and prefer the
glossary's chosen word over its listed _Avoid_ synonyms.

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

Hexagonal (Ports & Adapters), destined to become a **Cargo workspace**: a pure
domain crate plus one crate per adapter. The hexagonal cut is enforced **at the
crate level** — the domain crate depends on no I/O crate (no `reqwest`, no
`rusqlite`) and therefore *cannot* do I/O
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
  Validate once at the edge; the type then carries the proof. No raw
  `String`/`f64` crosses the DTO boundary into the domain.
- **Sum types with exhaustive `match`**, not a flag plus optional fields:
  `Richtung::Eingang|Ausgang`, `Gegenkonto::Eigen|Fremd`,
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
  step). The load-bearing first test asserts that the projection
  `Buchung → KategorieAnfrage` **carries no structural identifier**; it is the
  template for every later slice test.
- Constructor validation is tested too: an illegal value yields **no object**.

## Build order (tracer-bullet, vertical slices)

Build in this order; each step leaves something runnable/valuable:

1. **Cargo workspace + domain skeleton** — `Buchung` from value objects + the
   narrow `KategorieAnfrage` outbound type, with the no-structural-identifier
   test. _(Replaces the old Python hello-world.)_ Starts on explicit "Go".
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
