# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Architektur-Umbau im Gange ([ADR-0007](docs/adr/0007-modularer-monolith-feature-slices.md)).**
> Das Repo wird von „ein geteiltes `domain`-Crate + Adapter" auf einen **modularen
> Monolithen aus Feature-Slices** umgestellt: pro Slice eine Triade
> `<slice>_domain`/`_adapter`/`_specs`, dazu ein **dünner** interface-only
> `shared_kernel`. Das geteilte `domain`-Crate wird aufgelöst; jeder Slice hat
> eigene Value Objects und gibt nach außen nur DTOs heraus. Die „Current state"-
> und Crate-Listen unten beschreiben teils noch den Vor-Umbau-Stand und werden
> phasenweise nachgezogen — maßgeblich ist ADR-0007.

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

**Build-order steps 1 (domain skeleton) and 2 (Eigene-Konten-Config +
CSV-CAMT-v8 adapter) are implemented; steps 3+ are not started.** The repo is a
**Cargo workspace** with three members: the pure domain crate
[`crates/domain`](crates/domain) (package `domain`, edition 2024), the
[`crates/eigene_konten`](crates/eigene_konten) config crate, and the
[`crates/csv_camt_v8`](crates/csv_camt_v8) inbound adapter. The hexagonal cut is
real at the crate level: `domain` depends on no I/O crate — only `serde` (for the
outbound `KategorieAnfrage` DTO) and `static_assertions` (dev); the adapter
crates depend on `domain`, never the reverse. An earlier Python/`uv` scaffold was
deliberately replaced ([ADR-0003](docs/adr/0003-rust-als-sprache.md)); ignore any
lingering Python references.

Implemented in the domain crate:

- **Value objects** with validating `new → Option` constructors (_parse, don't
  validate_): `Betrag` (unsigned + `Währung`), `KontoId`, `Pseudonym`,
  `GläubigerId`, `Fingerabdruck`, `ObfuskierterText` — the non-empty string
  newtypes share one validating core, `NonEmptyString`, and are built from it
  via `From`.
- **Sum types** with exhaustive `match`: `Richtung::Eingang|Ausgang`,
  `Gegenkonto::Eigen(Rolle)|Fremd` (an own counter-account carries its
  `Konto-Rolle` — "eigen without a role" is unconstructible; the role must ride
  along at map time because obfuscation discards the counter-IBAN),
  `Empfänger::Firma|Person`, `Buchungsart`. `Rolle` is UBQL and lives in the
  domain; `eigene_konten` re-exports it.
- `Buchung` composed from those — `verwendungszweck: Option<ObfuskierterText>`
  (an absent free-text is modelled at the field, not as an empty
  `ObfuskierterText`) — plus the pure projection `Buchung → KategorieAnfrage`
  (whose `verwendungszweck` is likewise `Option`).
- The structural-identifier invariant is enforced **at compile time**:
  **placeholder** `Iban`/`Bic` ([strukturelle_identifikatoren.rs](crates/domain/src/strukturelle_identifikatoren.rs))
  derive no `Serialize`, proven by a `static_assertions` `assert_not_impl_any!`
  — an embedding attempt is a compile error, not a runtime test. The real,
  checksum-validating `Iban` now lives in `eigene_konten` (also deriving no
  `Serialize`); the domain placeholders keep guarding the invariant, and `Bic`
  stays a placeholder until a format needs it.

Implemented in the adapter crates:

- **`eigene_konten`** — the Eigene-Konten-Config (per own account: IBAN +
  `Rolle`) behind the `ConfigSource` port; the single source for
  `Gegenkonto::Eigen|Fremd` resolution and the analysis's role interpretation.
- **`csv_camt_v8`** — the `Umsatzquelle` inbound port + the CSV-CAMT-v8 adapter.
  Reads CSV behind the fs-free `CsvQuelle` port and maps each row to a `Buchung`,
  **obfuscating while mapping** (raw IBAN/BIC never reach a `Buchung`):
  `Auftragskonto → KONTO_x` (first-seen, run-stable), Gegen-IBAN → `Gegenkonto`
  via the config, Empfänger → `Firma`/`Person` (fail-safe), free-text →
  `ObfuskierterText`, one currency per file enforced.

Deferred: `ObfuskierterText` only trims and rejects empty (best-effort
**scrubbing** of embedded identifiers comes with step 5, ADR-0003);
`Fingerabdruck` deduplicates on the Empfänger identity only (no
payment-intermediary detection yet); `PERSON_x` is only run-stable (the
persistent pseudonym mapping is step 4). No binary yet.

All three crates opt into the workspace's strict `[workspace.lints.clippy]` (the
"Don't Panic" set) via `[lints] workspace = true`; `cargo test`, `cargo clippy
--all-targets`, and `cargo fmt` are green. Next: **step 3** (analysis core:
recurrence + fees + cashflow + Pareto — valuable without the LLM).

## Language convention

The project language is **German**: docs, the glossary, commit messages, and the
**domain vocabulary in code** are German. Code identifiers split by whether they
carry that vocabulary:

- **Domain vocabulary (the ubiquitous language / UBQL, per
  [CONTEXT.md](CONTEXT.md)) stays German** — types, their fields, noun-getters,
  and every name _component_ that is a glossary term: `Buchung`, `Betrag`,
  `Empfänger`, `Verwendungszweck`, `Buchungstext`/`Buchungsart`, `Gläubiger-ID`,
  `Fingerabdruck`, `Umbuchung`, `Gebühr`, `Richtung`, `Gegenkonto`,
  `Kategorie(Anfrage)`, `Währung`, … — do not anglicize them, and prefer the
  glossary's chosen word over its listed _Avoid_ synonyms. "Structural
  identifiers" (`strukturelle_identifikatoren`) counts as UBQL too — it names the
  privacy concept CONTEXT.md defines.
- **Every identifier that does NOT carry domain vocabulary is English** — in
  particular constructors and utility/conversion methods, purely technical helper
  types, and generic local/parameter names. Concretely: `::neu` is a no-go →
  `::new`; likewise `as_str`, `inner`, `resolve`, `identity_key`, the shared
  non-empty-text core `NonEmptyString`, and locals like `value`/`trimmed`. A
  compound name is translated per component — keep the glossary noun, anglicize
  the generic verb/preposition: `aus_buchung` → `from_buchung`,
  `zu_kategorie_anfrage` → `to_kategorie_anfrage`.
- **The deciding test: "must a domain expert read/understand this name?"** — yes
  ⇒ UBQL, German; no (pure code mechanics) ⇒ English. It cuts _within_ a type: a
  struct `Kontoauszug` and its domain action `Kontoauszug::drucken()` stay
  German, but the technical `Kontoauszug::parse()` is English. **Function
  internals** — locals, parameters, and technical helpers no domain expert reads
  — are English **even inside a German-named function**:
  `eingabe`/`normalisiert`/`länge`/`zeichen`/`länderkürzel_ok`/`prüfziffern_ok`
  → `input`/`normalized`/`length`/`c`/`country_code_ok`/`check_digits_ok`. The
  four architectural **ports** are a German-named category of their own
  ([ADR-0001](docs/adr/0001-hexagonale-architektur.md): `Umsatzquelle`,
  `Kategorisierer`, `Speicher`, `Bericht`); a new port and its immediate
  vocabulary join it in German (e.g. `Konfigquelle`), even where the concept is
  infrastructural.
- **`#[test]` function names are fully German** — a test is living documentation
  a domain expert reads, so the whole name is a German specification sentence,
  verbs included: `negativer_betrag_ist_nicht_konstruierbar`,
  `empfänger_ohne_gläubiger_id_wird_fail_safe_als_person_aufgelöst`,
  `unbekannte_rolle_wird_abgelehnt`, `iban_mit_falscher_prüfsumme_wird_abgelehnt`.
  This is the one deliberate exception to the "non-UBQL ⇒ English" rule above —
  the name's job is to communicate the problem and the solution to a domain
  expert, not to be code mechanics. **Test-internal helpers and port fakes**
  (`fixture_konten`, an `iban(…)` builder, a `FileSource` fake, `dummy_path`) are
  plumbing no domain expert reads → they keep the normal split (English mechanics,
  German glossary nouns). Only German _artifact prose_ — doc comments and
  assertion/`expect` messages — stays German everywhere.
- **The crate/package is named `domain`** (English): it carries no glossary
  vocabulary, so the rule applies to it like any other non-UBQL identifier.

- Die **Ausgabesprache des Agenten** (Claude) ist in diesem Repository immer
  **Deutsch** — unabhängig davon, in welcher Sprache der Nutzer schreibt. Das
  gilt für alle Chat-Antworten des Agenten, zusätzlich zur obigen Regel, dass
  Docs, Glossar, UBQL-Code-Bezeichner und Commit-Messages Deutsch sind (nicht vom
  UBQL abhängige Bezeichner dagegen Englisch, z. B. `::neu` → `::new`);
  domänenspezifische Begriffe aus [CONTEXT.md](CONTEXT.md) bleiben davon
  unberührt (weiterhin exakt wie dort definiert, nicht eindeutschen).

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
