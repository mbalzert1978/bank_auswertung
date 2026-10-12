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
> Fehlschlag surfacet, beantwortet seit
> [Issue 0085](docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md)
> der **Broker**: er sondert die Nachricht aus — Status-Spalte in der Outbox, der Slice merkt nichts). **Alle drei** I/O-Schritte jedes Slice — lesen, ablegen,
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
- [docs/adr/](docs/adr/) — seventeen ADRs (0008/0009 governing the pipeline/kernel — refined
  by **0011** (built): each slice decoupled from the Outbox mechanics, kernel cut flipped and
  merged into `application_kernel`; and by **0012**, which fixes the pipeline to
  **durchweg terminal** — no dead-letter/retry in the slice, fallible read/store/publish —
  and pins the **uniform ID handoff** (DB-assigned new ID as the published pointer, foreign
  entities referenced by ID only). **0013** (*accepted*) adds the **`fenstern`** step and the
  `auswerten` split — two analysis reaches, per-window and window-spanning; **0014**
  (*accepted*) pins the real `io` persistence to **rusqlite** and keeps the Outbox spine
  **synchronous** (`sqlx` rejected — async ripple through `application_kernel`/all slices vs.
  ADR-0003); **0015** (*accepted*, **built** — issues 0074–0077) collapses the four `io`
  result-store hulls into **one** generic `SliceStore<S>` over a `StoreSpec` trait — purely
  `io`-internal mechanics, **no** slice↔`io` seam changed; **0016** (*accepted*, **built** — issue
  0088) gives every `einlesen` line a **row key** (a new import adds, a known line is neither stored
  nor published again), binds windows to **one run** (run id from the `StartAnalysis` trigger,
  window-spanning query scoped by it) and skips `Umsatz vorgemerkt`; **0017** (*accepted*, **built** —
  issue 0085) answers 0012's deferred question: the broker **dead-letters** a failed message via a
  `status` column in the outbox (terminal at once, transient after 3 attempts), runs are never blocked
  by it, and a dead-lettered `Ingested` marks every later analysis **incomplete**; 0010 governing seed-based pseudonym
  stability; 0001–0007 each carry a superseded banner pointing at the canonical successor).
- [docs/prd/0001-bank-auswertung.md](docs/prd/0001-bank-auswertung.md) — PRD
  (mirrors the README; adds user stories + the module breakdown).

**Cargo workspace** (edition 2024; every member inherits the strict
`[workspace.lints.clippy]` "Don't Panic" set via `[lints] workspace = true`) —
**fifteen crates under `crates/`** plus the top-level validator crate
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
  dispatcher, and the per-slice wiring — plus the **real** adapters: the SQLite Outbox broker
  + `UnitOfWork`, **all four** slice result stores and **every** read seam of the chain, the
  CSV-CAMT-v8 source, the `Calendar`/`Money`/`Currency` decoding, the seed-keyed obfuscator and
  the `toml` loaders for the window-size and the seed config (io-Persistenz = **rusqlite**, die
  Outbox-Spine bleibt **synchron** —
  [ADR-0014](docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)). Since
  [ADR-0015](docs/adr/0015-generischer-slice-store-ueber-storespec.md) the four result stores
  each stand as a `StoreSpec` `impl` on **one** generic `SliceStore` in
  [`sqlite/slice_store.rs`](crates/io/src/sqlite/slice_store.rs). Since
  [Issue 0083](docs/issues/0083-lauf-anstoss-binary-reiht-den-ausloeser-ein-und-dreht-die-broker-loop.md)
  it also holds [`io::run`](crates/io/src/run/mod.rs) — the wiring of **all** real adapters into one
  program. The **category register** is real too since
  [Issue 0009](docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)
  ([`io::auswerten::SqliteCategoryRegistry`](crates/io/src/auswerten/registry.rs)). The `Kategorisierer`
  oracle is a **chat** since [Issue 0011](docs/issues/0011-kategorisierer-chat-payload-adapter.md)
  (export at the end of a run, import as its own command). The **HTML report** lives in `io` too since
  [Issue 0012](docs/issues/0012-html-bericht.md) (`crates/io/src/report/`, written by `io::run` at the
  end of an exit-0 run);
- [`lauf`](crates/lauf) — the **bin crate** (`bank-auswertung`), pure command line: eight mandatory
  path options in (plus the optional `--kategorien-export` and the `kategorien-import` subcommand,
  issue 0011), exit code out. Binds `io` as its **only** internal and **no** external dependency
  (asserted by the `architecture` test);
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
  `CalendarSeam`/`MoneySeam`, `io`-fulfilled by `GermanCalendar`/`GermanMoney`/`IsoCurrencies`)
  decode German date/number, and an *undecodable* present value returns `None`, which the
  aggregate maps to a terminal
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
  a **pure trigger** (empty `RunConfiguration`, field-less payload). The envelope
  `partition_key` **names** the source (its file name) but does not **select** it: the batch is
  fetched over the data-source port this run was wired with, so a redelivered trigger would read
  whatever file the *current* invocation points at. That is why `io::run` refuses to start when an
  open trigger names a *different* source (`RunError::PendingImport`) — without it, the leftover of
  an aborted run silently became a second import of the current file, at exit code 0. (Since 0085 a
  trigger that **failed** is dead-lettered, not left open — only a crashed run leaves one open.) The handler
  **fetches** the batch through the
  domain-required data-source port (`RawLineSource`, fallible + terminal), which the
  `SourceBridge` binds to the `SourceSeam` (`io`-fulfilled by the real `CsvCamtV8Source`).
  Fans out **one `Ingested` message per valid line**; read, store and publish are each **fallible +
  terminal**, and a terminal error (fetch/presence/decode, or a failing store/publish)
  surfaces as the slice's **naked, redacted terminal domain error** — the pipeline is
  **durchweg terminal** (no dead-letter/retry in the slice; `io`'s broker dead-letters the message,
  [ADR-0012](docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md), issue 0085).
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
  All free text (purpose, booking text, company name) passes the **`ObfuskierterText` scrubber**
  ([Issue 0010](docs/issues/0010-scrubbing-regeln-fuer-obfuskiertertext.md)): IBAN patterns,
  BIC patterns and **known person names** become fixed placeholders (`[IBAN]`/`[BIC]`/`[PERSON]`) —
  **heuristic, not provably complete**, fail-safe. The name list comes in as a value over the
  `KnownPersonsSource` port; `io` fills it per booking from `einlesen_lines.recipient_name` (rows
  **without** creditor ID — so a company without one is blacked out too, the known cost) plus the
  local `personen.toml`. Only **whole** names match: a recipient's first name alone, a title or one
  half of a double name is **not** blacked out automatically — those go into `personen.toml`. Names
  **do not act retroactively**: a recipient first seen in a later import stays readable in purposes
  obfuscated earlier (no re-scrub, by design) — so fill `personen.toml` with all known recipient
  names of all accounts (incl. first names/short forms) **before the first run**. Side effect: a
  card-payment merchant without creditor ID lands on the list too; if its name appears in a payment
  intermediary's purpose it becomes `[PERSON]`, and two such merchants behind one intermediary fall
  onto the **same fingerprint**. Persistent seed management deferred.
- **`fenstern`** — third pipeline step and the **paginating analysis head**
  ([ADR-0013](docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)); the first
  **N→1** slice. **Two single-purpose handlers** (the type carries the intent, ADR-0005): the
  `StartHandler` consumes `StartAnalysis` (no anchor — only the **run id** its sender minted,
  ADR-0016, which then travels through every `AdvanceWindow` into every window row and the
  `HistorieVollständig`), reads the initial anchor once over the
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
  the periodic metric **within** the window — today the **month cashflow**, **netted** — plus the
  window's **distinct fingerprints**, and stores the result. The **window-spanning** reach
  (`HistoryEvaluationHandler`, consuming `HistorieVollständig` — a **different** dispatch key, scoped
  to the run by the run id (ADR-0016) in the `partition_key`, next to the run span) runs **one** read-port query over **all** windows
  of the run and **returns** the occurrences of the whole timeline **plus the recurring outflows**
  in them — it **stores nothing** (the aggregates are the sole truth; idempotence follows from
  querying immutable snapshots, not dedupe logic). The **recurrence detection**
  ([Issue 0005](docs/issues/0005-wiederkehr-erkennung.md), **built**) is pure domain logic: per
  cost driver (fingerprint, or the fees of one account — since 0007 one grouping shared with the
  ranking), **outflows only** and **netted** (a standing order onto an own account is a transfer,
  not a cost driver — same token rule and `SeedMarker` guard as the cashflow), it finds a monthly
  (gap 25–35 days, ≥ 3 charges) or yearly (350–380 days, ≥ 2) rhythm at a similar amount (±5 % per
  price level) and reports the latest **price increase** (cut-off day, old/new). Result per group:
  `RecurrenceRow` (driver, rhythm, typical amount, **annual amount** — the yardstick for 0007 —,
  price increase, charged account token) inside the returned `HistoryEvaluationRow`, guarded
  no-`Serialize` as an egress candidate for report/export. The **`Fingerabdruck`** is a **pure domain function** (company via `GläubigerId`, payment
  intermediary additionally via the merchant token, private person via the seed-based pseudonym) —
  domain, not adapter or `io`. **`Gegenkonto::Eigen|Fremd` is resolved on tokens** against the
  token-oriented own-accounts config passed in as a value (`io` wires it; the slice reads no file, no
  raw IBAN ever reaches it), with a `SeedMarker` guard that fails the run terminally if window and
  config were not tokenised under the same seed. On that resolution rests the **`Umbuchung` netting**
  ([Issue 0004](docs/issues/0004-umbuchung-netting.md), **built**): a booking onto an own account is
  no outflow, so it leaves the cashflow — **leaves, not vanishes**. It stands next to it as an
  `InternalTransfer`, per month and counter-account role, so the gross flow is the sum of both views.
  Netting is decided **per row, not per pair**: that the counter account is mine already stands on the
  single booking, and waiting for its second leg would count every transfer whose counterpart sits in
  another window or an unimported statement as an expense. Next to it stands the **fee
  classification** ([Issue 0006](docs/issues/0006-gebuehren-klassifikation.md), **built**), and the two
  pull in **opposite directions** — which is the point: a `BookingKind::Fee`, derived by a pure domain
  function from the bank-assigned booking text, is real lost money, so it **stays** in the cashflow and
  is merely lifted out **alongside** it as a `Fee` per month and **charged account** (an overlay, a
  subset of `months` — not a second half like the transfers, and adding the two would double-count).
  The fee markers are deliberately conservative and still **HITL-unverified** against Markus' real
  statements. Calls `Kategorisierer` only for **unknown**
  fingerprints; its egress DTO carries the no-`Serialize` guard. Read/store/query each **fallible +
  terminal**, **no** publish seam (it is the pipeline tail until `report`). On top of both sits the
  **Aderlass** ([Issue 0007](docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md), **built**),
  on the **window-spanning** reach because only it sees the whole run (a per-window Pareto would rank one
  ~400-day page): the netted cashflow **per own account and month** (`AccountCashflowRow`, role from the
  own-accounts config), the **Pareto ranking** of cost drivers (`CostDriver::Kind(fingerprint) |
  Fee(charged account) | Unattributed`, grouped **once** — recurrence and ranking come from the same
  grouping, so a monthly account fee is recurring too; shares in basis points, only the cumulative share
  is rounded, so all shares sum to exactly 10 000) and
  the **savings candidates**: every driver except the unattributed lump whose **annual amount** reaches
  the configured threshold (`--sparen`, decision V3) — recurring → from the recurrence, otherwise the
  per-year average over the observed span, never extrapolated upwards. All returned in the
  `HistoryEvaluationRow`, guarded no-`Serialize`, nothing stored, no schema change. Because this reach
  sums across windows, the read seam delivers each window's currency and a run with more than one is
  rejected terminally (`EvaluationError::MixedCurrencies`); the row carries the one `currency`.

(An earlier `eigene_konten` triad was removed: the ADR-0008 pipeline moved
`Gegenkonto::Eigen|Fremd` resolution into the **`auswerten`** slice, done **on
tokens** — the counter-account `KONTO_x` against the seed-tokenised set of own accounts —
so a raw-IBAN register consumed at ingest no longer fits. The own-accounts config
(IBAN + role Giro/Haushalt/Spar/Vorlauf) is now an `auswerten` input: since issue 0042 the slice
takes the **token-oriented** own-accounts config as a value (`io` wires it, the slice reads no
file), and the config loader that turns raw IBANs into tokens is issue 0002, still deferred.)

**Status.** Green: `cargo build`/`test`/`clippy --all-targets`/`fmt --check` +
`cargo test -p architecture` + `cargo doc --no-deps --workspace`. Built: `application_kernel` +
the `io`-private Outbox seams, the generic transactional consumer/dispatcher in `io`, and **all
four slices, each wired through `io`**: `einlesen`, the `obfuskieren` slice (seed-based), the
`fenstern` slice (the paginating analysis head — two consumers: kick-off + window step), and the
`auswerten` split (**both reaches** built — per-window and window-spanning; the `Fingerabdruck`
domain function, token-based `Gegenkonto` resolution, and the `Kategorisierer` port for unknown
fingerprints) — the whole `ADR-0013` chain (issues 0036–0045) is done.

**Real `io` adapters** (io-Persistenz = **rusqlite**, Outbox-Spine bleibt **synchron**, `sqlx`
verworfen — [ADR-0014](docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)):

- **Spine + decoding:** the SQLite Outbox broker + `UnitOfWork` (0052), the
  `Calendar`/`Money`/`Currency` decoding (0050/0055), the CSV-CAMT-v8 source (0051) and the
  seed-keyed obfuscator (0056).
- **Result store per slice — all four real:** `einlesen` (0053), `obfuskierung` (0056),
  `fenstern` (0057), `auswerten` (0058); since
  [ADR-0015](docs/adr/0015-generischer-slice-store-ueber-storespec.md) each stands as a
  `StoreSpec` `impl` on **one** generic `SliceStore` (issues 0074–0077).
- **Read source per slice — all real:** `EinlesenRowSource` (obfuskierung, 0056),
  `ObfuskiertWindowSource` (fenstern, 0057) and `AuswertungsfensterSource` (auswerten, 0058).
- **Config loaders:** window size (0054) and seed (0059), both `toml`, both living centrally in
  [`io::config`](crates/io/src/config/mod.rs).

So **no** slice is missing its store or its read source any more. `einlesen` runs its integration
test without a single fake; the three downstream wiring tests still drive fakes for the unit of
work (their real building blocks are covered by the crate-internal store/source tests against
real SQLite).

The **Eigene-Konten loader** is real too since issue 0083
([Issue 0002](docs/issues/0002-eigene-konten-config-schema-und-loader.md), io side):
`io::config::EigeneKontenConfig` reads `[[konto]]` (IBAN + role), rejects a bad form/mod-97 checksum,
an unknown role, a missing field or a duplicate — **redacted**, never quoting the IBAN — and hands
`auswerten` the **token-oriented** config, tokenised with the same obfuscator/seed as the bookings.
The raw IBAN never leaves it (no getter, redacted `Debug`). Only 0002's **HITL** part is left: Markus'
real IBANs in the gitignored `eigene-konten.toml`.

**The category register is real** ([0009](docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)):
`io::auswerten::SqliteCategoryRegistry`, one table `learned_categories` keyed on the **fingerprint**
(never on text), holding the learned label per kind; a correction replaces the old label and survives
every restart. It is the **only** source of truth for labels: the chat import (0011) writes solely via
`learn(&[CategoryAnswerRow])` (all or nothing, redacted rejection, and only fingerprints some evaluation
already carries — an invented key such as a plain name never reaches the file); the report (0012) reads
solely via `categories_of`. The label is free validated text, as the domain's `Category` already is. It
came with schema **version 3**, the first rung of the migration **ladder** in `io::schema` (each rung one
version up, stamped once at the end).

**The `Kategorisierer` is a chat** ([0011](docs/issues/0011-kategorisierer-chat-payload-adapter.md)) —
the first data leaving the machine. In the run, `io::auswerten::CollectingCategorizer` collects the
domain's requests (unknown kinds only, from phase 2 on) and answers empty — ADR-0004's litmus test
stays: without the LLM the program still computes everything. `--kategorien-export <file>` writes them
at exit 0 (`0600`, built by hand from two fields, no `Serialize`); `bank-auswertung kategorien-import
--db … --antwort … [--korrektur]` learns the answer via `learn`, matched **only** on the exact
fingerprint, known labels kept unless `--korrektur`. What goes out is chosen by the domain: the
counterparty is classified once (`Party`: intermediary / company / person / none); intermediary → only
the purpose, company → only the name, **persons never** (so card payments get no chat label), and one
**egress word rule** drops every word with a digit or `@` before splitting. The creditor ID is
**parsed** (form + mod 97) in `obfuskieren` (an invalid one makes the counterparty a person) **and**
again at `auswerten`'s ACL (a row from an older file with an invalid one ends the run terminally).
The `report`
is **built** (issue 0012): HTML in `io`, its display DTO carries the no-`Serialize` guard, accounts
appear by role, persons only as pseudonym, companies by their scrubbed name, categories only from the
register (else „unbekannt"). Also deferred: persistent seed management
([ADR-0010](docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md),
[Issue 0008](docs/issues/0008-persistente-pseudonym-zuordnung.md) — the seed **loader** is real,
persisting the seed across runs is not). Note what the `obfuskiert` table stores: **not the seed but
its marker**, the opaque seed-keyed hash `auswerten` compares window against own-accounts config.
The seed itself never touches disk — were it in the table, every `KONTO_x` would be brute-forceable
against a plaintext list by anyone holding the file, which is the whole pseudonymisation undone.

**Still open beyond the adapters** (issues filed 2026-07-29):

- ~~**No program at all**~~ — **built** ([0083](docs/issues/0083-lauf-anstoss-binary-reiht-den-ausloeser-ein-und-dreht-die-broker-loop.md)):
  `bank-auswertung` enqueues the triggers and turns the broker loop against the real adapters,
  writing a redacted run log to stderr. Two gaps the ticket did not foresee, both resolved in
  [`io::run`](crates/io/src/run/mod.rs) and documented there: (a) **import and analysis are not
  message-coupled** — nobody consumes `Obfuscated`, nobody publishes `StartAnalysis` — so the run has
  **two phases** and acks `Obfuscated` through an `AckSink`; (b) registry and oracle had no
  fulfilment at all, hence the empty-answering adapters above.
- ~~**No schema owner, no version stamp**~~ — **built**
  ([0084](docs/issues/0084-schema-an-einem-ort-und-versionsstempel-als-migrationsachse.md)):
  [`io::schema::open`](crates/io/src/schema.rs) is the **one** place a database gets its schema (outbox
  + all four result stores, in one transaction) and stamps it via `PRAGMA user_version`
  (`SCHEMA_VERSION`). Same version → used as is; any other version — including an unstamped file that
  already has tables (version 0) — fails the run terminally and redacted (`RunError::SchemaVersion`,
  versions only). The stores no longer create tables (`SliceStore::new` is DDL-free; their own-table
  creation is a `#[cfg(test)]` path). A migration *n → n+1* is one more arm in `schema::ensure` — the
  first one, **1 → 2**, came with ADR-0016 (row key + run id; a version-1 file **with** data is
  rejected, `MismatchCause::UnkeyedRows`).
- ~~**Repeat runs give wrong numbers**~~ — **built**
  ([0088](docs/issues/0088-zeilen-schluessel-lauf-eigene-fenster-und-vorgemerktes-ueberspringen.md),
  [ADR-0016](docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)): row key, run-owned
  windows, `Umsatz vorgemerkt` skipped — see „Many runs, one `--db`" below.
- ~~**No dead-letter**~~ — **built**
  ([0085](docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md)),
  the answer to what [ADR-0012](docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)
  deferred ([ADR-0017](docs/adr/0017-dead-letter-als-status-spalte-der-outbox.md)): the outbox has a
  **`status` column** (`open`/`processed`/`dead_letter`, ladder rung 3 → 4, schema **v4**) and counts
  attempts. The broker ([`sqlite/delivery.rs`](crates/io/src/sqlite/delivery.rs)) dead-letters a
  **terminal** failure (`Warning`/`Critical`) at once and a **transient** one (`Retry`, a failed
  commit) after `MAX_DELIVERY_ATTEMPTS` (3, a constant), reporting from the row it hit (`RETURNING`);
  the redacted reason stays on the row (`SqliteOutbox::dead_letters`). Both run phases drain anyway; a
  run that dead-letters something itself names it and ends with `RunError::DeadLettered` (exit 1); a
  later run is not blocked, counts earlier ones as „früher ausgesondert", and reads from the **file**
  how many bookings are missing (dead-lettered `Ingested`) → `RunReport::incomplete`, „Analyse
  unvollständig" in the log, exit 0. No slice changed.
- Cosmetic: the `toml` dependency carries semver build metadata, so every workspace command
  prints a warning ([0086](docs/issues/0086-toml-abhaengigkeit-ohne-semver-metadaten.md)).

**Next**, two threads (weighed in
[docs/plans/2026-07-31-1335](docs/plans/2026-07-31-1335-aufwand-und-reihenfolge-der-offenen-posten.md)):

- **Towards an answer** — the `Umbuchung` netting ([0004](docs/issues/0004-umbuchung-netting.md)) and
  the fee classification ([0006](docs/issues/0006-gebuehren-klassifikation.md)) are **done**, so the
  cashflow numbers are now right rather than merely computed, and what the bank itself takes is named
  separately. On top of them the
  Pareto/savings-candidate analyzer ([0007](docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md))
  is **built** (candidate = annual amount over a config value, decision V3), and so is the `report`
  ([0012](docs/issues/0012-html-bericht.md)) — the program now says **where the money goes**. One **HITL** item is now open on 0006
  too: which booking texts his bank actually sets for fees.
- **Towards a safe operation** — the schema owner + version stamp
  ([0084](docs/issues/0084-schema-an-einem-ort-und-versionsstempel-als-migrationsachse.md)) and the
  dead-letter ([0085](docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md))
  are both **built**: a file on disk and a poisoned message each have an owner.

The `Kategorisierer` chat export/import (0011) and the recurrence analyzer (0005) are built.

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

A real run (issue 0083) — all eight options are mandatory, the run log goes to stderr:

```bash
cargo run -p lauf -- --db lauf.sqlite --csv <auszug>.csv   --fenster fenster.toml --seed seed.toml --eigene-konten eigene-konten.toml --personen personen.toml --sparen sparen.toml --bericht bericht.html
```

`--bericht` is the HTML report (issue 0012): written only when the run ends with exit 0, for exactly
the run id that run minted (ADR-0016); after exit 1 an older file there stays untouched.

`--sparen` is the savings threshold (issue 0007; copy `sparen.example.toml`, not secret):
`jahresbetrag_ab_cent` — from this annual amount on, a cost driver is proposed as a savings candidate.

`--personen` is the local name list for the free-text scrubber (issue 0010; copy
`personen.example.toml`, gitignored). It is **mandatory even when empty** (`namen = []`): a forgotten
list stops the run instead of silently scrubbing fewer names.

Exit code `0` = the outbox ran empty and the report is written (log and report still say „Analyse
unvollständig" if an earlier run dead-lettered a booking), `1` = this run dead-lettered a message (named, redacted — the message is out
of delivery and does not block later runs), `2` = the command line was wrong. The invented fixtures under
[`crates/io/fixtures/lauf/`](crates/io/fixtures/lauf/) plus
[`lauf.csv`](crates/io/fixtures/einlesen/csv/lauf.csv) are a runnable example.

**Many runs, one `--db`** ([ADR-0016](docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md),
[0088](docs/issues/0088-zeilen-schluessel-lauf-eigene-fenster-und-vorgemerktes-ueberspringen.md)) — one
run per own account, repeats included, into the **same** database. A new import **adds**: every line
carries a **row key** (ordering account + end-to-end reference + amount + booking day when the
reference is filled, otherwise all columns; plus the running number of equal **booked** lines within
the file — `Umsatz vorgemerkt` gets no key), stored as a SipHash digest, and a known key is neither
stored nor published again. Every run still re-paginates the
**whole** obfuscated history (all accounts), but its windows carry its **run id**, and the
window-spanning query scopes over the run — so the **latest** run shows all accounts together and
the same run repeated gives the same numbers (the old 53 → 159 → 318 growth is gone). „The latest
run" is the one with the **greatest** `run_id` (it starts with the run's UTC start time). `Umsatz
vorgemerkt` lines are skipped before the presence check. **Limit (both key paths):** a day must
stand **completely** in an export — otherwise equal lines of that day could be taken for known ones. A
leftover trigger from a *failed* run is still caught (`RunError::PendingImport`). Schema version is
**2**; a version-1 file without data is migrated, one **with** data is rejected (its lines cannot be
keyed retroactively) — use a fresh `--db` and re-import.
Git worktrees go under `.claude/worktrees/` (local-only, gitignored).

## Architecture — the big picture

Hexagonal (Ports & Adapters) als modularer Monolith aus Feature-Slice-Triaden; Pipeline
`einlesen → obfuskieren → fenstern → auswerten → report`, jeder Schritt ein Outbox-Consumer,
`obfuskieren` die **Privacy-Grenze**, `auswerten` die Analyse (das LLM nur Orakel für
**unbekannte** Fingerabdrücke). Der `Speicher` ist **kein eigener Port**, sondern die Summe der
SQLite-Ablagen in `io`: die vier Slice-Stores (je hinter dem Seam ihres Slice, sie speichern **DTOs**,
nicht Domänenobjekte) plus das Kategorie-Register `SqliteCategoryRegistry` (hinter dem
`CategoryRegistrySeam`, die einzige Wahrheit für Kategorie-Etiketten). Der zweite Rand ist die
`report`-HTML-Ausgabe. Die **normativen Prinzipien** — Ports,
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
   free text; the privacy boundary. **done** — real end-to-end since issues 0056/0059 (the
   `einlesen` read seam, the seed-keyed obfuscator, the `obfuskiert` store and the seed config
   loader) and the `ObfuskierterText` scrubbing rules since issue 0010 (IBAN/BIC patterns + known
   names, heuristic); still needs persistent seed management
   ([ADR-0010](docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)).
4. **`fenstern` slice** — consume `StartAnalysis`/`AdvanceWindow`, paginate the obfuscated
   history newest→oldest (`max(booking_day < from)`) and **materialise one
   `Auswertungsfenster` aggregate per page**; publish `FensterGeschlossen` per page and
   `HistorieVollständig` on exhaustion
   ([ADR-0013](docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).
   **done** — issues 0036–0039, real store + read seam since 0054/0057.
5. **`auswerten` slice** — **two single-purpose handlers**, **both built**: per-window
   (today the **netted month cashflow** *within* one `Auswertungsfenster`, carrying the window's
   distinct fingerprints) and window-spanning (recurrence *reach* as a **read-port query over all
   windows of the run**, triggered by `HistorieVollständig` — no fold-log). Owns the `Fingerabdruck`
   domain function and token-based `Gegenkonto::Eigen|Fremd` resolution (counter `KONTO_x` vs. the
   seed-tokenised own-accounts config, IBAN + role). Calls `Kategorisierer` only for unknown
   fingerprints. Issues 0040–0045 **done**, real store + both read seams since 0058; the first
   analyzer on top of them, the **`Umbuchung` netting** (0004), is **built** — the cashflow now
   answers „where do we bleed" instead of counting own transfers as spending, and what it drops
   stands next to it in `window_evaluation_transfers`. The **second**, the **fee classification**
   (0006), is built too and pulls the other way: a fee is real lost money, so it **stays** in the
   cashflow and is only lifted out alongside it into `window_evaluation_fees`, per month and charged
   account. The **third**, the **recurrence detection** (0005), is built on the window-spanning
   reach: netted outflows per fingerprint → monthly/yearly rhythm, price increase, annual amount,
   returned (never stored) as `RecurrenceRow`s. The **fourth**, the **Aderlass** (0007), is built on the
   same reach: cashflow per own account and month, Pareto ranking of cost drivers, savings candidates
   over the `--sparen` threshold — returned in the `HistoryEvaluationRow`, never stored.
   `Kategorisierer` still comes in from the caller as a fake.
6. **`Speicher` (SQLite)** — no port of its own, but the four slice stores plus
   `SqliteCategoryRegistry` — and the real **`io`** adapters
   fulfilling the seams — **largely built**: the SQLite Outbox broker/`UnitOfWork`, the
   `Calendar`/`Money`/`Currency` seams, the CSV-CAMT-v8 source, **all four** slice result stores
   (on **one** generic `SliceStore`,
   [ADR-0015](docs/adr/0015-generischer-slice-store-ueber-storespec.md)), **every** read source
   of the chain and the `toml` loaders for window size and seed are real (issues 0050–0059,
   0074–0077) — **and the Eigene-Konten loader** (issue 0002, io side, with issue 0083).
   **And the category register** (issue 0009, schema version 3). **And the chat
   export/import** of the `Kategorisierer` (issue 0011).
7. **The run itself** — **done for the kick-off** (issue 0083): the `lauf` bin crate + `io::run`
   enqueue the triggers and turn the broker loop against the real adapters; the pipeline now runs
   outside tests. The one schema owner + version stamp is **built** (0084), and so is the
   dead-letter that keeps a poisoned message from being redelivered forever (0085).
8. **`report`** HTML output — **built** (issue 0012), in `io`, at the end of an exit-0 run.
9. **Iteration** — review together, learn corrections, name savings.

Prerequisites before their step (see README "Offen" / PRD) — **data**, no longer adapters: a
real CSV-CAMT-v8 sample file for a real `einlesen` run; a **stable config seed** (its `toml`
loader is real since issue 0059, the *value* is still the user's to supply) + the local
**`personen.toml`** (names never a recipient, plus first names/short forms of recipients — filled
before the first run, since names do not act retroactively; the scrubbing rules themselves are built,
issue 0010) before a real `obfuskieren` run
([ADR-0010](docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)); the **Eigene-Konten-Config**
(per own account: IBAN + role Giro/Haushalt/Spar/Vorlauf — single source for
`Gegenkonto::Eigen|Fremd`) **plus its loader** (issue 0002) before the `auswerten` run.

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
