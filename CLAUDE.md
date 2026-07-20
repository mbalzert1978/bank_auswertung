# CLAUDE.md

Guidance for Claude Code (claude.ai/code) working in this repo.

> **Architektur:** modularer Monolith aus **Feature-Slice-Triaden**
> (`<slice>_domain`/`_adapter`/`_specs`), gekoppelt als **event-getriebene Pipeline
> über eine Outbox**. Maßgeblich:
> **[ADR-0008](docs/adr/0008-event-getriebene-pipeline-und-outbox.md)** (Pipeline aus
> Outbox-Consumern) — verfeinert durch
> **[ADR-0013](docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)**
> (**akzeptiert**), das den Schritt **`fenstern`** einzieht, sodass die Pipeline
> `einlesen → obfuskieren → fenstern → auswerten → report` lautet; jeder Schritt ist ein
> Outbox-Consumer. Dazu
> **[ADR-0009](docs/adr/0009-kernel-split-und-idiomatische-outbox-vertraege.md)**
> (Rust-idiomatische Outbox-Verträge; sein Kernel-**Split** ist inzwischen wieder
> zusammengeführt, s. u.). Alle verfeinern/supersedieren
> [ADR-0001](docs/adr/0001-hexagonale-architektur.md)–[0007](docs/adr/0007-modularer-monolith-feature-slices.md)
> (jedes Alt-ADR trägt oben einen Superseded-Banner mit kanonischem Verweis).
>
> **[ADR-0011](docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)** ist
> **gebaut**: der Slice ist eine reine `Command → Domäne → Response`-Pipeline;
> Consumer-Rolle, Fehler-Taktik, `UnitOfWork` und Envelope-Kenntnis leben **vollständig in
> `io`** (generischer `TransactionalHandler`/Consumer; der Slice liefert nur den
> Envelope-Decoder). Der Kernel-Schnitt aus 0009 ist **geflippt und zusammengeführt**:
> `application_kernel` ist der **einzige** Slice↔`io`-Vertrag, die Outbox-Seams liegen
> **`io`-privat** — die frühere Crate `shared_kernel` wurde **aufgelöst**
> ([Issue 0032](docs/issues/0032-shared-kernel-io-privat-machen-und-crate-in-io-aufl-sen-adr-0011.md)).
> Der `<slice>_adapter` bindet nur noch `application_kernel`.
> Auch das **0012-Slice-Muster ist gebaut**: `einlesen`/`obfuskierung` sind durchweg
> terminal mit fehlbarem read/store/publish und uniformem ID-Handoff (Issues 0028/0029,
> gemerged).
>
> **[ADR-0012](docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)**
> korrigiert die Fehler-Taktik: die Pipeline ist **durchweg terminal** — **kein**
> Dead-Letter, **kein** Retry im Slice (die frühere „`io` klassifiziert
> Warning/Retry/Critical"-Erzählung war ein Missverständnis; wie `io` einen terminalen
> Fehlschlag surfacet, ist vertagt). **Alle drei** I/O-Schritte jedes Slice — lesen, ablegen,
> publizieren — sind **fehlbar**; der Slice gibt seinen **nackten, redigierten** terminalen
> Domänen-Fehler zurück. Das **ID-Handoff ist uniform**: jeder Slice liest den Vorgänger per
> ID, schreibt in seine **eigene** neue Tabelle (die Persistenz vergibt die ID) und
> publiziert **diese neue** ID als Zeiger; Aggregate referenzieren Fremd-Entitäten **nur per
> ID**. `einlesen` ist der Kopf (Datei hinter dem Seam, per Auslöser getriggert).

## What this is

**Bank-Auswertung** — local, maintainable Rust software that reads bank-account
transactions and surfaces the main cost drivers and realistic savings potential.
**Pure analysis**: never triggers transactions, never changes account data.

**Docs-first** — read the docs before implementing:

- [README.md](README.md) — source of truth for **scope and the privacy promise** only
  (deliberately free of architecture/crate detail so it cannot go stale); the architecture
  is owned by [docs/adr/](docs/adr/) and this file.
- [CONTEXT.md](CONTEXT.md) — domain glossary (the ubiquitous language).
- [docs/adr/](docs/adr/) — thirteen ADRs (0008/0009 governing the pipeline/kernel — refined
  by **0011** (built): each slice decoupled from the Outbox mechanics, kernel cut flipped and
  merged into `application_kernel`; and by **0012**, which fixes the pipeline to
  **durchweg terminal** — no dead-letter/retry in the slice, fallible read/store/publish —
  and pins the **uniform ID handoff** (DB-assigned new ID as the published pointer, foreign
  entities referenced by ID only). **0013** (*accepted*) adds the **`fenstern`** step and the
  `auswerten` split — two analysis reaches, per-window and window-spanning; 0010 governing
  seed-based pseudonym stability; 0001–0007 each carry a superseded banner pointing at the
  canonical successor).
- [docs/prd/0001-bank-auswertung.md](docs/prd/0001-bank-auswertung.md) — PRD
  (mirrors the README; adds user stories + the module breakdown).

**Cargo workspace** (edition 2024; every member inherits the strict
`[workspace.lints.clippy]` "Don't Panic" set via `[lints] workspace = true`) —
**fourteen crates under `crates/`** plus the top-level validator crate
[`architecture`](architecture):

- **one thin, interface-only kernel crate** ([ADR-0009](docs/adr/0009-kernel-split-und-idiomatische-outbox-vertraege.md),
  cut flipped and merged by [ADR-0011](docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):
  [`application_kernel`](crates/application_kernel) — the **only** slice↔`io` contract
  (`Command`/`Handler`/`Publisher`/`PublishCommand`/`CommandDecoder`), dependency-free,
  bound by the slice adapters and `io`;
- four **feature-slice triads** `<slice>_domain`/`_adapter`/`_specs`:
  [`einlesen`](crates/einlesen_adapter), [`obfuskierung`](crates/obfuskierung_adapter),
  [`fenstern`](crates/fenstern_adapter), [`auswerten`](crates/auswerten_adapter);
- [`io`](crates/io) — holds the **io-private Outbox seams**
  (`OutboxMessage`/`OutboxCommand`/`OutboxError`/`OutboxHandler`/`OutboxPublisher`/
  `UnitOfWork` — here since [Issue 0032](docs/issues/0032-shared-kernel-io-privat-machen-und-crate-in-io-aufl-sen-adr-0011.md)
  dissolved the former `shared_kernel` crate), the generic transactional consumer +
  dispatcher, and the per-slice wiring with in-memory doubles. The real `sqlx`/`csv`/`toml`
  adapters are still deferred;
- [`architecture`](architecture/tests/dependency_direction.rs) — test-only crate
  (outside `crates/`) asserting dependency direction over `cargo_metadata`.

(An earlier Python/`uv` scaffold and the earlier shared `domain` crate were removed
— ignore any lingering Python or `crates/domain` references.)

**Hexagonal cut enforced three ways:** (1) crate level — a `<slice>_domain` crate is
**dependency-free** (not even `serde`/`thiserror`; errors are hand-written `Display` +
`std::error::Error`, mapped by the adapter), and a slice reaches a foreign slice only
through its public `_adapter`, never its `_domain`; (2) the `architecture` test
(`cargo test -p architecture`) fails on any violated inward edge (domain not blind,
external dep outside `io`, foreign-`_domain` coupling, specs not black-box, only
`_adapter` binds `application_kernel`); (3) the `enforce-dependency-direction.py`
PreToolUse hook blocks a non-sanctioned external `[dependencies]` line at edit time in
every crate except the deferred `io` crate and `architecture`.

Per slice (value objects live in the owning slice; only DTOs cross the boundary):

- **`einlesen`** — head of the pipeline ([ADR-0008](docs/adr/0008-event-getriebene-pipeline-und-outbox.md));
  named for its responsibility (*read + validate + record*), **not** its format
  (CSV-CAMT-v8 is one input form behind the source seam; XML-camt/MT940 later =
  further io adapters, not new slices). `_domain` owns `LineAggregate` + its value
  objects and **demands already-decoded, complete values** — presence lives in the
  **type**: `RawLine`'s mandatory fields are non-`Option` (ADR-0005: illegal states
  unconstructible, not "checked"). **Presence enforcement is the `_adapter` (ACL)**,
  not the domain and not `io` — since [Issue 0018](docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
  **behind the source seam in the `SourceBridge`**: `RawLine::try_from(&RawRecord)` is fallible;
  a missing/empty mandatory cell maps to the **redacted** domain
  `SourceError::MissingMandatoryField` (static field name only, never a raw value) and
  fails the whole batch terminally, before the handler interprets a single line. The domain does not
  validate format either: the `Calendar`/`Money` ports (behind the adapter's
  `CalendarSeam`/`MoneySeam`, `io`-fulfilled later) decode German date/number, and an
  *undecodable* present value returns `None`, which the aggregate maps to a terminal
  `LineError` (now only `AmountInvalid`/`CurrencyUnknown`/`BookingDayInvalid`/
  `ValueDateInvalid`; `CalendarDay::from_parts` is infallible — the seam owns calendar
  validity). `recipient_name` is genuinely **optional** (a fee/charge row has no
  counterparty). `_adapter` exposes the `RawRecord` inbound DTO (**no longer `serde`**
  since [0018](docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md):
  it is the **source-seam DTO**, fetched in-process, not a wire payload), the `Line`
  outbound projection, the Consume-/Publisher-/Source-/Port-Bridges, and the Fluent
  `ReadTestApi`. It **deliberately re-declares** the domain sum types it projects
  (`Direction`/`Currency`/`CalendarDay`/`Sign`) as adapter-owned DTO types instead of
  re-exporting them — no domain internal crosses the boundary (re-evaluate the mapping
  concentration at the `io` revision). Consumes a `ReadTransactions` command — since
  [0018](docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
  a **pure trigger** (empty `RunConfiguration`, field-less payload; the source lives in
  the envelope `partition_key`): the handler **fetches** the batch through the
  domain-required data-source port (`RawLineSource`, fallible + terminal), which the
  `SourceBridge` binds to the `SourceSeam` (fake now, `io` later). Fans out **one
  `Ingested` message per valid line**; read, store and publish are each **fallible +
  terminal**, and a terminal error (fetch/presence/decode, or a failing store/publish)
  surfaces as the slice's **naked, redacted terminal domain error** — the pipeline is
  **durchweg terminal** (no dead-letter/retry in the slice; how `io` surfaces a terminal
  failure is deferred, [ADR-0012](docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)).
- **`obfuskierung`** — second pipeline step and the **privacy boundary**: a consumer that
  reads the raw aggregate **by ID** from the `einlesen` store (`RawBookingSource`) and
  obfuscates it **seed-based** — a deterministic `KONTO_x`/`PERSON_x` token from
  `(seed, plaintext)` via the `Obfuscator` port it requires, **no** First-Seen counter and
  **no** plaintext↔token mapping store
  ([ADR-0010](docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)). The slice binds **no
  `serde`**; `_adapter` exposes the `IngestedRow` inbound DTO and the `ObfuscatedBooking`
  egress projection carrying the `assert_not_impl_any!` no-`Serialize` guard, plus a Fluent
  TestApi with in-memory fakes. Consumes `Ingested` (envelope = ID pointer); read, store and
  publish are each **fallible + terminal** (a straight `?`-chain). It reads the raw booking
  by ID, obfuscates, writes the `obfuskiert` table — **the store assigns the new ID** — and
  publishes **that new ID** (the `Obfuscated` message) as the pointer to `auswerten`; the obfuscated aggregate
  references the source booking **by ID only** (foreign key), never as an embedded raw
  object. A missing mandatory field or an unreadable/absent source row is a **terminal,
  redacted** failure (static field name / opaque ID, never a raw value) — **no retry in the
  slice** ([ADR-0012](docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)).
  Persistent seed management deferred.
- **`fenstern`** — third pipeline step and the **paginating analysis head**
  ([ADR-0013](docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)); the first
  **N→1** slice. **Two single-purpose handlers** (the type carries the intent, ADR-0005): the
  `StartHandler` consumes the field-less `StartAnalysis`, reads the initial anchor once over the
  `WindowAnchor` port (`max(booking_day)`, data-relative, no clock) and publishes the first
  `AdvanceWindow` — an **empty store is no error** (the run ends `Ok`, publishes nothing); the
  `WindowingHandler` then, per command, spans the page `[anchor − window_days + 1 … anchor]`,
  **reads** its obfuscated rows, **reduces** them N→1 into one `Auswertungsfenster` aggregate
  (invariants at construction: **non-empty** + **one currency**), **stores** it (the store assigns
  the new ID) and **publishes that new ID** as the pointer (`FensterGeschlossen`). It then **continues
  itself**: the next anchor is `max(booking_day < from)` — a day found → next `AdvanceWindow`, none
  → `HistorieVollständig` with the run span. Read/store/publish each **fallible + terminal** (ADR-0012).
  Downstream of the privacy boundary, so it sees only opaque IDs, one currency and day-stamps — no
  egress DTO, hence no `assert_not_impl_any!` guard. Page size `WindowDays` and the day-stamp axis
  come in as **values** (`io` reads the config; the domain binds no calendar).
- **`auswerten`** — fourth pipeline step and **the analysis itself**
  ([ADR-0004](docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md): the domain is the
  analysis, the LLM only an oracle). **Two reaches, two single-purpose handlers — both built**
  (ADR-0013, decision E). The **per-window** reach (`WindowEvaluationHandler`, consuming
  `FensterGeschlossen` — an **ID pointer**) reads the frozen `Auswertungsfenster` by ID, computes
  the periodic metric **within** the window — today the **month cashflow** — plus the window's
  **distinct fingerprints**, and stores the result. The **window-spanning** reach
  (`HistoryEvaluationHandler`, consuming `HistorieVollständig` — a **different** dispatch key, scoped
  to the run by the run span in the `partition_key`) runs **one** read-port query over **all** windows
  of the run and **returns** the occurrences of the whole timeline — it **stores nothing** (the
  aggregates are the sole truth; idempotence follows from querying immutable snapshots, not dedupe
  logic). The **`Fingerabdruck`** is a **pure domain function** (company via `GläubigerId`, payment
  intermediary additionally via the merchant token, private person via the seed-based pseudonym) —
  domain, not adapter or `io`. **`Gegenkonto::Eigen|Fremd` is resolved on tokens** against the
  token-oriented own-accounts config passed in as a value (`io` wires it; the slice reads no file, no
  raw IBAN ever reaches it), with a `SeedMarker` guard that fails the run terminally if window and
  config were not tokenised under the same seed. Calls `Kategorisierer` only for **unknown**
  fingerprints; its egress DTO carries the no-`Serialize` guard. Read/store/query each **fallible +
  terminal**, **no** publish seam (it is the pipeline tail until `report`). Still to come on top of
  these reaches: the fee/Pareto/`Umbuchung`-netting/recurrence analyzers (building blocks 0004–0007).

(An earlier `eigene_konten` triad was removed: the ADR-0008 pipeline moved
`Gegenkonto::Eigen|Fremd` resolution into the **`auswerten`** slice, done **on
tokens** — the counter-account `KONTO_x` against the seed-tokenised set of own accounts —
so a raw-IBAN register consumed at ingest no longer fits. The own-accounts config
(IBAN + role Giro/Haushalt/Spar/Vorlauf) is now an `auswerten` input: since issue 0042 the slice
takes the **token-oriented** own-accounts config as a value (`io` wires it, the slice reads no
file), and the config loader that turns raw IBANs into tokens is issue 0002, still deferred.)

**Status.** Green: `cargo build`/`test`/`clippy --all-targets`/`fmt --check` +
`cargo test -p architecture`. Built: `application_kernel` + the `io`-private Outbox seams,
the generic transactional consumer/dispatcher in `io`, and **all four slices end-to-end against
fakes, each wired through `io`**: `einlesen`, the `obfuskieren` slice (seed-based), the `fenstern`
slice (the paginating analysis head — two consumers: kick-off + window step), and the `auswerten`
split (**both reaches** built — per-window and window-spanning; the `Fingerabdruck` domain function,
token-based `Gegenkonto` resolution, and the `Kategorisierer` port for unknown fingerprints)
— the whole `ADR-0013` chain (issues 0036–0045) is done. Deferred:
the real `io` adapters (`csv`/`toml`/`fs`/`sqlx` + SQLite Outbox broker/stores + `Speicher`),
the `report` presenter, the remaining egress-DTO `Serialize` guards, `ObfuskierterText`
scrubbing (only trims/rejects-empty today), and persistent seed management
([ADR-0010](docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)). **Next: the real `io`
adapters (SQLite Outbox broker/stores, file/CSV parse, config loaders) fulfilling the seams, then
the `report` HTML presenter.** No binary yet.

## Language convention

Project language is **German** → all docs and tests in German. Everything else,
including code, in **English**.

## Commands

Standard Cargo, from the repo root:

```bash
cargo build                      # compile
cargo test                       # all tests
cargo test <name>                # single test by name match
cargo clippy --all-targets       # lint
cargo fmt                        # format
cargo doc --no-deps --workspace  # docs — sixth gate: rustdoc-Warnungen sind harte Fehler
```

The **six gates** that must be green: `build`, `test`, `clippy --all-targets`,
`fmt --check`, `cargo test -p architecture`, and `cargo doc --no-deps --workspace`.
`cargo doc` is a real gate, not a nicety: `[workspace.lints.rustdoc]` sets
`private_intra_doc_links`/`broken_intra_doc_links` to `deny`, so öffentliche Doku, die auf
ein privates oder nicht existierendes Item verlinkt, **bricht den Build** statt nur zu
warnen (Issue 0048). Ein Kollaborateur darf weiter **benannt** werden (Code-Auszeichnung
oder Prosa) — nur nicht als toter Link, der keiner werden kann.

No binary yet (library crate, no `src/main.rs`), so `cargo run` does nothing useful.
Git worktrees go under `.claude/worktrees/` (local-only, gitignored).

## Architecture — the big picture

Hexagonal (Ports & Adapters) as a **modular monolith of feature-slice triads**. The
cut is enforced **at the crate level**: a `<slice>_domain` crate is dependency-free and
depends on no I/O crate (no `reqwest`/`rusqlite`), so it *cannot* do I/O — guarded by
the `architecture` test and the PreToolUse hook.

**The domain is the analysis; the LLM is only an oracle**
([ADR-0004](docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)). Reading,
obfuscating, presenting are adapter jobs. The pure domain does: recurring-outflow
detection (incl. yearly costs + price increases), transfer (`Umbuchung`) netting, fee
classification, cashflow per account/month, Pareto ranking, savings candidates, and the
category register with learned corrections. **Litmus test**: remove the LLM entirely and
the program is still valuable — only automatic labels for unknown merchants are missing.

**The pipeline ([ADR-0008](docs/adr/0008-event-getriebene-pipeline-und-outbox.md), refined by
[ADR-0013](docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)):**
`einlesen → obfuskieren → fenstern → auswerten → report`. Each step is its own slice triad and an
**Outbox consumer** — handles one incoming command and writes, in **one transaction**,
both its result table and an Outbox message for the next step (atomic, at-least-once,
resumable). No synchronous next-call; coupling is the message alone. `obfuskieren` is
the **privacy boundary** (raw before, tokens after); `auswerten` is the analysis and, on
an **unknown** fingerprint, calls the injected `Kategorisierer` port itself (LLM oracle,
export → chat → import) — a **domain decision**, not a pipeline step. Two edge ports:
`Speicher` (SQLite, stores **DTOs**, not domain objects) and the `report` HTML output
(shows the domain's analysis, not "what the LLM said").

Another input format (XML-camt, MT940) = new **io adapter behind `einlesen`'s source
seam** — no new slice, no domain change; another categorizer/output = new adapter behind
an existing port. Out of scope now (YAGNI).

## Non-negotiable invariants

The reason the project exists; do not weaken them.

**Privacy / obfuscation** ([ADR-0008](docs/adr/0008-event-getriebene-pipeline-und-outbox.md),
[ADR-0001](docs/adr/0001-hexagonale-architektur.md), [ADR-0003](docs/adr/0003-rust-als-sprache.md)):

- Structural identifiers (own IBAN/account no., counter-IBAN, BIC) **never leave the
  machine**. The boundary is **`obfuskieren`, not the ingress**: raw may **exist and
  persist locally** up to that step (the `einlesen` table + `einlesen → obfuskieren`
  message carry raw — no different from the `.csv` on disk); from `obfuskieren` on, only
  tokens flow. So the compiler-enforced `static_assertions` "no `Serialize`" guard
  belongs on the **egress DTOs** (categorizer request + report output — where data
  leaves): embedding a raw identifier in something that leaves is a compile error, not a
  review miss. It is **not** on `einlesen`'s raw `Iban` (raw never reaches an egress DTO;
  since [0018](docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
  it no longer rides an outbox message at all — the batch is fetched **in-process**
  through the source seam and lives only in the local `einlesen` table); today the guard
  sits on `obfuskierung`'s egress `ObfuscatedBooking` (the privacy-boundary output).
  `serde` is allowed — the guarantee rests on the missing `Serialize` on egress types.
  `Bic` deferred until a format needs it.
- From `obfuskieren` on, the downstream domain (`auswerten`) receives **no** person-/
  account-relatable identifier — only obfuscated value objects. The **`Gläubiger-ID`**
  (public company key) is a deliberate exception, kept as an analysis anchor.
- Free-text (Empfänger/Verwendungszweck) must pass through the single value object
  **`ObfuskierterText`**, whose constructor best-effort scrubs embedded identifiers —
  **heuristic, not provably complete**; state that honestly, don't claim anonymization.
- **Fail-safe** Firma vs. Privatperson: when unsure, treat as `Person` and pseudonymize.
  The costlier error is leaking a real name.

**DTO boundary / Anti-Corruption-Layer**
([ADR-0002](docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)): the domain's only
public surface is DTOs. **Each port speaks its own narrow DTO**, never a shared god-DTO.
The `Kategorisierer` port speaks `KategorieAnfrage` (fingerprint + `ObfuskierterText`)
outbound, `KategorieAntwort` (fingerprint → category) inbound — the answer keys on the
**fingerprint, never the text**, so re-identification is structurally impossible.

**Fingerprint is domain knowledge**
([ADR-0004](docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md),
[ADR-0006](docs/adr/0006-pseudonym-stabilitaet.md)): `Buchung → Fingerabdruck` is a pure
domain function (incl. payment-intermediary detection + merchant token), **not** an
adapter concern. The LLM is asked only about fingerprints the domain does not yet know.

**The local secret stays local** (never let it reach an egress DTO — that is a leak path)
([ADR-0010](docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md), supersedes ADR-0006's
"two separate stores"): pseudonym stability rests on a config-injected **seed**, not a
persistent plaintext↔pseudonym mapping store. The seed (persisted with each obfuscated
aggregate for rehydration) and the raw `einlesen` table may live locally; the obfuscated
`Speicher` and every egress DTO carry **neither the seed nor a raw identifier**. Same seed
→ same `PERSON_x` across imports, so the fingerprint of person-to-person standing orders
stays stable; an unstable seed re-tokenises everything.

## Domain model conventions ([ADR-0005](docs/adr/0005-domaenenmodell-value-objects-und-summentypen.md))

- **Value objects, not primitives** (_parse, don't validate_): `Betrag`, `KontoId`,
  `Pseudonym`, `Fingerabdruck`, `GläubigerId`, `ObfuskierterText`. The non-empty string
  newtypes share one validating core, `NonEmptyString` (trim + reject-empty), built from
  it via `From`. Validate once at the edge; the type carries the proof. No raw
  `String`/`f64` crosses the DTO boundary into the domain.
- **Sum types with exhaustive `match`**, not a flag + optional fields:
  `Richtung::Eingang|Ausgang`, `Gegenkonto::Eigen(Rolle)|Fremd`, `Empfänger::Firma|Person`,
  `Buchungsart`. Illegal combinations must be **unconstructible**, not "checked".
- `Betrag` is **unsigned** with a currency; the sign lives only in `Richtung` (a negative
  `Eingang` is unconstructible). The domain does **not** sum mixed currencies — the
  inbound adapter guarantees a single currency.
- Style **declarative-functional**: pure transformations over immutable values,
  `map`/`filter`/`fold` over manual loops, `match` over `if/else` cascades. Side effects
  at the edges (adapters).

## Testing ([ADR-0002](docs/adr/0002-dto-grenze-und-anti-corruption-layer.md), [ADR-0008](docs/adr/0008-event-getriebene-pipeline-und-outbox.md))

Two rules govern *what* to test — the codebase's test culture:

- **The domain does not validate — it demands.** It asks for already-valid,
  already-decoded, complete values. But *who* enforces what splits three ways: **format
  decode is `io`'s job** (behind `Money`/`Calendar`: German number/date) — so **do not
  test format-mechanics for rejection** (sign-less amount, unreadable date, broken
  payload); that tests `io`/kernel, not the slice. **Presence enforcement is the
  `_adapter` (ACL)** — the domain demands presence in the type (mandatory field
  non-`Option`), the adapter's inbound mapping rejects a missing one and **fails the batch
  terminally**. That is **observable slice behaviour, so specs *do* test it** (a missing
  mandatory field → redacted, batch-atomic terminal failure). A **domain decode decision**
  (`Currency::from_code`: is this a currency the domain knows?) is likewise slice
  behaviour and testable. And a genuine **domain invariant** triggered by bad input stays
  (e.g. "the terminal failure reason never mirrors a raw value" — redaction). What
  stays out of the specs is only pure `io`-crate mechanics (reading a file, real
  decoding).
- **Test behaviour, not implementation.** Behaviour runs **through the Fluent `*TestApi`**:
  arrange a DTO, run the real bridge chain against fakes, and the domain objects get built
  — "if something's wrong, it blows up", so the domain is covered transitively.
  Implementation churns, so **no** unit tests nailing constructor/algorithm internals
  (`Currency::from_code`, `Amount::with_direction`, aggregate assembly) — remove them,
  don't move them into the spec. A test of a genuine standalone rule (`NonEmptyString`:
  trim + reject-empty; IBAN mod-97 checksum) *may* stay but is redundant (covered via
  TestApi).

Concretely: **`*_specs` are pure black-box** over the slice's one public adapter surface
(Fluent `*TestApi` + DTOs + the slice's own naked domain error, re-exported by the adapter —
no Outbox types), never internals, no DTO-constructor tests in the spec. They test real
behaviour — happy path (line → command + staged aggregate), fan-out, redaction, and the
**rejections that are slice behaviour**
(a missing mandatory field → batch-atomic terminal failure; an unknown currency) — but **not**
rejection of pure `io` format-mechanics (unreadable date, sign-less amount). A wrapper VO
with guaranteed-valid input (`From<NonEmptyString>` only) and a logicless port/fake get
**no** test (pure noise).

## Build order (pipeline flow order)

Build in flow order ([ADR-0008](docs/adr/0008-event-getriebene-pipeline-und-outbox.md),
refined by [ADR-0013](docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md));
each step runnable/valuable against fakes, real `io` last:

1. **Kernel + Outbox contracts** (`application_kernel` + the `io`-private Outbox seams) — **done**.
2. **`einlesen` slice** — `RawRecord → LineAggregate → Ingested`, batch in / one message
   per valid line, German date/number behind the `Calendar`/`Money` seams. **done** (fakes).
3. **`obfuskieren` slice** — consume `Ingested`, assign seed-based `KONTO_x`/`PERSON_x`, scrub
   free text; the privacy boundary. **done** (fakes); still needs the `ObfuskierterText`
   scrubbing rules + persistent seed management
   ([ADR-0010](docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)).
4. **`fenstern` slice** — consume `StartAnalysis`/`AdvanceWindow`, paginate the obfuscated
   history newest→oldest (`max(booking_day < from)`) and **materialise one
   `Auswertungsfenster` aggregate per page**; publish `FensterGeschlossen` per page and
   `HistorieVollständig` on exhaustion
   ([ADR-0013](docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).
   **done** (fakes) — issues 0036–0039.
5. **`auswerten` slice** — **two single-purpose handlers**, **both built** (fakes): per-window
   (today the **month cashflow** *within* one `Auswertungsfenster`, carrying the window's distinct
   fingerprints) and window-spanning (recurrence *reach* as a **read-port query over all windows of
   the run**, triggered by `HistorieVollständig` — no fold-log). Owns the `Fingerabdruck` domain
   function and token-based `Gegenkonto::Eigen|Fremd` resolution (counter `KONTO_x` vs. the
   seed-tokenised own-accounts config, IBAN + role). Calls `Kategorisierer` only for unknown
   fingerprints. Issues 0040–0045 **done**; the fee/Pareto/`Umbuchung`-netting/recurrence analyzers
   (building blocks 0004–0007) build *on top* of these reaches and are still to come.
6. **`Speicher` (SQLite)** for DTOs + category register, and the real **`io`** adapters
   (SQLite Outbox broker/stores, file/CSV parse) fulfilling the seams.
7. **`report`** HTML output.
8. **Iteration** — review together, learn corrections, name savings.

Prerequisites before their step (see README "Offen" / PRD): a real CSV-CAMT-v8 sample file
for a real `einlesen` run; a **stable config seed** + **scrubbing rules** for
`ObfuskierterText` before a real `obfuskieren` run
([ADR-0010](docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)); the **Eigene-Konten-Config**
(per own account: IBAN + role Giro/Haushalt/Spar/Vorlauf — single source for
`Gegenkonto::Eigen|Fremd`) before the `auswerten` run.
