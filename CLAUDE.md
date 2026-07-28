# CLAUDE.md

Guidance for Claude Code (claude.ai/code) working in this repo.

Diese Datei trägt **Projekt-Narrativ, Struktur und Baustand**. Die normativen
**Coding-Regeln/Konventionen** leben zentral in [.rules/](.rules/) (Format: kurze Aussage +
Do/Don't) und werden am Ende dieser Datei per `@`-Import automatisch geladen — siehe den
Abschnitt **„Regeln & Konventionen"**. Regeln dort pflegen, nicht hier duplizieren.

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
- [docs/adr/](docs/adr/) — fourteen ADRs (0008/0009 governing the pipeline/kernel — refined
  by **0011** (built): each slice decoupled from the Outbox mechanics, kernel cut flipped and
  merged into `application_kernel`; and by **0012**, which fixes the pipeline to
  **durchweg terminal** — no dead-letter/retry in the slice, fallible read/store/publish —
  and pins the **uniform ID handoff** (DB-assigned new ID as the published pointer, foreign
  entities referenced by ID only). **0013** (*accepted*) adds the **`fenstern`** step and the
  `auswerten` split — two analysis reaches, per-window and window-spanning; **0014**
  (*accepted*) pins the real `io` persistence to **rusqlite** and keeps the Outbox spine
  **synchronous** (`sqlx` rejected — async ripple through `application_kernel`/all slices vs.
  ADR-0003); 0010 governing seed-based pseudonym stability; 0001–0007 each carry a superseded
  banner pointing at the canonical successor).
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
  dispatcher, and the per-slice wiring — **`einlesen` now runs end-to-end against real
  `rusqlite`/`csv` adapters** (SQLite Outbox broker + `UnitOfWork`, the `einlesen` store, the
  CSV-CAMT-v8 source), plus a `toml` loader for the `fenstern` window-size config; the
  remaining three slices (`obfuskierung`/`fenstern`/`auswerten`) still wire against in-memory
  doubles (io-Persistenz = **rusqlite**, die Outbox-Spine bleibt **synchron** —
  [ADR-0014](docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md));
- [`architecture`](architecture/tests/dependency_direction.rs) — test-only crate
  (outside `crates/`) asserting dependency direction over `cargo_metadata`.

(An earlier Python/`uv` scaffold and the earlier shared `domain` crate were removed
— ignore any lingering Python or `crates/domain` references.)

**Hexagonaler Schnitt, dreifach erzwungen** (Crate-Ebene, `architecture`-Test,
PreToolUse-Hook) — die Regel mit Do/Don't in
[.rules/crates/dependencies-und-kapselung.md](.rules/crates/dependencies-und-kapselung.md).

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
  object. A missing mandatory field, an absent source row or an **unreachable** read store is a
  **terminal, redacted** failure — and the three stay **distinguishable** on the domain error
  (static field name / opaque ID / passed-through io diagnosis, never a raw value): since issue
  0078 the read seam carries the same three-valued shape as `auswerten`'s (`Err` = fetch failed,
  `Ok(None)` = nothing there), so a broken database no longer reports as "not found". **No retry
  in the slice** ([ADR-0012](docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)).
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
— the whole `ADR-0013` chain (issues 0036–0045) is done. The real `io` adapters have started
landing (issues 0050–0054, io-Persistenz = **rusqlite**, Outbox-Spine bleibt **synchron**,
`sqlx` verworfen —
[ADR-0014](docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)): the SQLite Outbox
broker + `UnitOfWork`, the real `Calendar`/`Money` seams, the CSV-CAMT-v8 source and the
`einlesen` store — so **`einlesen` now runs end-to-end without a single fake** — plus a `toml`
loader for the `fenstern` window-size config. Deferred: the `obfuskierung`/`fenstern`/
`auswerten` result stores (still in-memory doubles), the remaining config loaders (seed,
Eigene-Konten) and their `fs` adapters, `Speicher` for DTOs/category register, the `report`
presenter, the remaining egress-DTO `Serialize` guards, `ObfuskierterText` scrubbing (only
trims/rejects-empty today), and persistent seed management
([ADR-0010](docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)). **Next: the remaining
slice-specific `io` adapters (stores + config loaders) fulfilling the seams, then the `report`
HTML presenter.** No binary yet.

## Commands

Standard Cargo, from the repo root:

```bash
cargo build                      # compile
cargo test                       # all tests
cargo test <name>                # single test by name match
cargo clippy --all-targets       # lint
cargo fmt                        # format (gate: cargo fmt --check)
cargo doc --no-deps --workspace  # docs — sixth gate: rustdoc-Warnungen sind harte Fehler
```

Die **sechs Gates**, die grün sein müssen (inkl. `cargo doc` als echtes Gate), prüfst du
kanonisch über den Skill **`/cargo-six-gates`** — er führt alle sechs aus und meldet
PASS/FAIL je Gate. Definition und Do/Don't: [.rules/rust/gates.md](.rules/rust/gates.md).

No binary yet (library crate, no `src/main.rs`), so `cargo run` does nothing useful.
Git worktrees go under `.claude/worktrees/` (local-only, gitignored).

## Architecture — the big picture

Hexagonal (Ports & Adapters) als modularer Monolith aus Feature-Slice-Triaden; Pipeline
`einlesen → obfuskieren → fenstern → auswerten → report`, jeder Schritt ein Outbox-Consumer,
`obfuskieren` die **Privacy-Grenze**, `auswerten` die Analyse (das LLM nur Orakel für
**unbekannte** Fingerabdrücke). Zwei Rand-Ports: `Speicher` (SQLite, speichert **DTOs**, nicht
Domänenobjekte) und die `report`-HTML-Ausgabe. Die **normativen Prinzipien** — Ports,
Domäne-ist-die-Analyse, Outbox-Kopplung, durchweg terminal, uniformes ID-Handoff — mit
Do/Don't in [.rules/common/architektur.md](.rules/common/architektur.md).

## Regeln (ausgelagert nach `.rules/`)

Die früheren Regel-Abschnitte leben jetzt als atomare Do/Don't-Regeln in [.rules/](.rules/)
(am Ende dieser Datei per `@`-Import geladen):

- **Privacy/Obfuskierung, DTO-Grenze, Fingerabdruck, lokaler Seed** →
  [.rules/common/privacy-invarianten.md](.rules/common/privacy-invarianten.md) — der Grund,
  warum das Projekt existiert; nicht abschwächen.
- **Domänenmodell** (Value Objects, Summentypen, `Betrag` vorzeichenlos, deklarativ) →
  [.rules/common/domaenenmodell.md](.rules/common/domaenenmodell.md).
- **Testkultur** (Domäne verlangt statt validiert; Black-Box über die `*TestApi`) →
  [.rules/common/testing.md](.rules/common/testing.md).

## Build order (pipeline flow order)

Build in flow order ([ADR-0008](docs/adr/0008-event-getriebene-pipeline-und-outbox.md),
refined by [ADR-0013](docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md));
each step runnable/valuable against fakes, real `io` last:

1. **Kernel + Outbox contracts** (`application_kernel` + the `io`-private Outbox seams) — **done**.
2. **`einlesen` slice** — `RawRecord → LineAggregate → Ingested`, batch in / one message
   per valid line, German date/number behind the `Calendar`/`Money` seams. **done** — and,
   since issues 0050–0053, real end-to-end (`Calendar`/`Money`, the CSV-CAMT-v8 source, the
   SQLite Outbox broker and the `einlesen` store), no fakes left in this slice.
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
   fulfilling the seams — **in progress**: the SQLite Outbox broker/`UnitOfWork`, the
   `Calendar`/`Money` seams, the CSV-CAMT-v8 source and the `einlesen` store are real
   (issues 0050–0053), plus a `toml` loader for the `fenstern` window-size config
   (issue 0054); the `obfuskierung`/`fenstern`/`auswerten` stores, the remaining config
   loaders (seed, Eigene-Konten) and `Speicher` itself are still open.
7. **`report`** HTML output.
8. **Iteration** — review together, learn corrections, name savings.

Prerequisites before their step (see README "Offen" / PRD): a real CSV-CAMT-v8 sample file
for a real `einlesen` run; a **stable config seed** + **scrubbing rules** for
`ObfuskierterText` before a real `obfuskieren` run
([ADR-0010](docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)); the **Eigene-Konten-Config**
(per own account: IBAN + role Giro/Haushalt/Spar/Vorlauf — single source for
`Gegenkonto::Eigen|Fremd`) before the `auswerten` run.

## Regeln & Konventionen

Die normativen Coding-Regeln liegen zentral in [.rules/](.rules/) (jede Regel: kurze
Aussage + Do/Don't). Die folgenden `@`-Importe laden sie jede Session automatisch — Regeln
**dort** pflegen, neue Regeln im selben Format ergänzen (siehe
[.rules/README.md](.rules/README.md)):

@.rules/common/architektur.md
@.rules/common/privacy-invarianten.md
@.rules/common/domaenenmodell.md
@.rules/common/testing.md
@.rules/common/sprache.md
@.rules/common/tickets.md
@.rules/common/kein-automatisches-memory.md
@.rules/rust/gates.md
@.rules/rust/idiome.md
@.rules/rust/nebenlaeufigkeit.md
@.rules/rust/performance.md
@.rules/rust/testing-werkzeuge.md
@.rules/rust/compiler-getriebenes-implementieren.md
@.rules/sql/schema-konventionen.md
@.rules/sql/queries-und-performance.md
@.rules/sql/migrationen.md
@.rules/crates/dependencies-und-kapselung.md
