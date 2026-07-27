# Generischer Slice-Store über `StoreSpec` — die vier `io`-Ergebnis-Store-Hüllen kollabiert

> **Status: Akzeptiert** (Entscheidung + Doku aus einem `/grill-with-docs`-Durchstich;
> **kein** Code — die Umsetzung ist ein Folge-Issue). Rein `io`-interne Mechanik: **kein**
> Seam-Vertrag Richtung Slice ändert sich, keine `CONTEXT.md`-Sprache entsteht (siehe
> **Bewusst kein Glossar-Begriff**).

Die vier realen Slice-Ergebnis-Stores
(`crates/io/src/{einlesen,obfuskierung,fenstern,auswerten}/store.rs`) wiederholen dasselbe
**6-teilige Skelett** über dem schon tiefen SQLite-Rückgrat aus
[`sqlite/mod.rs`](../../crates/io/src/sqlite/mod.rs) (`SqliteUnit`, `mint_id`,
`ensure_columns`, `sqlite_backed_unit!`): Store-Struct · `new` · `UnitFactory` ·
`Unit`-Newtype · Seam-Body-„Tanz" · `Row`. Slice-spezifisch ist einzig **SCHEMA + INSERT(s)
+ Row-Flatten**. Dieses ADR kollabiert das Skelett in **einen generischen Typ**
`SliceStore<S>`/`SliceUnit<S>`, parametrisiert über ein neues Trait **`StoreSpec`** — ohne
[ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md) (Outbox-Mechanik lebt
vollständig in `io`, `io`-intern erlaubt), [ADR-0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)
(durchweg terminal, DB-vergebene ID als Zeiger) oder
[ADR-0014](0014-io-persistenz-rusqlite-synchron-kein-sqlx.md) (rusqlite, synchrone Spine) zu
berühren.

Herkunft: Architektur-Review (`/improve-codebase-architecture`), **Kandidat 1** (Top-Empfehlung),
gegen Domänenmodell und ADRs/`.rules` gegrillt (`/grill-with-docs`).

## Problem

Das Rückgrat ist bereits **tief** — `SqliteUnit` (Staging + Commit + Publish/Ack in **einer**
Transaktion), `mint_id` (Preflight + DB-ID-Minting), `ensure_columns` (Audit-Spalten-Nachrüstung).
Trotzdem trägt jeder Slice die **volle Hülle** darüber erneut; die Docstrings sagen es wörtlich
(*„die Replikation des `fenstern`-Musters"*). Vier Kopien bedeuten: eine Konventions- oder
Mechanik-Änderung (etwa der Redaktions-Regel, der Audit-Spalten oder des Preflight-Verhaltens)
muss viermal nachgezogen werden — und **driftet**, wie an `obfuskierung`s abweichender
Fehler-Form schon geschehen (siehe **Entscheidung**, `StoreUnavailable`).

Zugleich ist der Seam-Vertrag Richtung Slice (`StoreSeam`/`ObfuscatedStoreSeam`/
`AnalysisWindowStoreSeam`/`EvaluationStoreSeam`, je im `_adapter` definiert) **bewusst
pro Slice** und darf **nicht** verwässert werden ([ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):
er trägt den slice-eigenen DTO und die slice-eigene Rückgabe (neue ID als Zeiger vs. `()` am
Pipeline-Ende). Die Abstraktion muss also **`io`-intern** kollabieren, ohne die vier
Seam-Signaturen anzufassen.

## Entscheidung

Ein neues, `io`-internes Trait **`StoreSpec`** und zwei generische Typen in einer **neuen**
Datei `crates/io/src/sqlite/slice_store.rs`:

```rust
type StagedRow = Box<dyn FnOnce(&Transaction<'_>) -> rusqlite::Result<()> + 'static>;

pub struct TableSpec {          // eine Tabelle mit IHREM DDL und IHREM Insert — beisammen
    pub name:   &'static str,   // new(): ensure_columns
    pub ddl:    &'static str,   // CREATE TABLE IF NOT EXISTS … (+ ihre CREATE INDEX)
    pub insert: &'static str,   // Ablage-Statement → Preflight
}

pub trait StoreSpec {
    type Dto;                                     // die Adapter-DTO (Line, ObfuscatedBooking, …)
    type Prepared;                                // die VOLLSTÄNDIGE, besitzende, id-LOSE Speicher-Form
    type Error;                                   // die adaptereigene StoreUnavailable
    fn tables() -> &'static [TableSpec];           // EINE Liste: anlegen + nachrüsten + preflighten
    fn flatten(dto: &Self::Dto) -> Result<Self::Prepared, Self::Error>;  // fehlbar, VOR dem Mint — der EINZIGE DTO-Kontakt
    fn stage(prepared: Self::Prepared, id: String) -> StagedRow;         // baut die id-TRAGENDE Row in EINEM Zug, dann Kopf+Detail-Inserts
    fn unavailable(err: rusqlite::Error) -> Self::Error;                 // redigierter Konstruktor
}

pub struct SliceStore<S> { /* SqliteOutbox + PhantomData<S> */ }
pub struct SliceUnit<S>(SqliteUnit, PhantomData<S>);
```

Die getroffenen Weichen im Einzelnen:

1. **Form: generischer Typ + Trait**, nicht Makro und nicht bloße Helfer-Funktion. `SliceStore<S>`
   trägt Struct/`new`/`UnitFactory` **einmal**; die generische Ablage-Primitive
   `SliceUnit<S>::store_dto(dto) -> Result<String, S::Error>` trägt den Seam-Body-„Tanz"
   (fehlbares `flatten` → Preflight-`prepare` **aller** `TableSpec::insert` → `mint_id` → `stage`,
   das die Row **mit** der frisch geminteten ID baut). Pro Slice bleibt nur: **ein**
   `StoreSpec`-`impl` (`TableSpec`-Liste + flatten + stage) + eine **3–5-zeilige** Seam-`impl`, die
   `store_dto` aufruft.

2. **`sqlite_backed_unit!` entfällt für diese vier.** Der Passthrough
   `OutboxPublisher`/`UnitOfWork` wird **einmal generisch** implementiert
   (`impl<S: StoreSpec> … for SliceUnit<S>`) statt viermal per Makro erzeugt.

3. **Rückgabe-Asymmetrie: immer minten, ID zurückgeben, Tail verwirft.** `store_dto` liefert
   **stets** `Result<String, _>`. `einlesen`/`obfuskierung`/`fenstern` reichen die ID als
   Publish-Zeiger durch; `auswerten` (Pipeline-Ende, publiziert nichts) mappt sie per
   `map(drop)` auf `()`. Die ID bleibt **DB-vergeben** ([ADR-0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)) —
   **ob** sie publiziert wird, ist Slice-Sache, nicht Store-Sache. Kein `Option`, kein zweiter
   Seam.

4. **Mehr-Tabellen einheitlich (1 bis 5 Tabellen) — und die Row trägt die ID von Geburt an.**
   `flatten` läuft **vor** dem Mint und ist der **einzige** DTO-Kontakt: es deutet die Adapter-DTO
   **vollständig** in eine besitzende, **id-lose** `Prepared` um, inklusive aller fehlbaren Schritte
   (u64→i64 der Beträge, `numbered_bookings`-Positionsvorlauf). Dann mintet `store_dto` die ID und
   übergibt sie an `stage(prepared, id)`, das die **SQL-fertige, id-tragende Row in einem Zug mit
   der ID** baut (`LineRow::new(id, fields)` / `StagedEvaluation::new(id, …)`) und **eine** Closure
   liefert, die Kopf- und alle Detail-Inserts (positionsgetreu per `enumerate`) fährt. So existiert
   **nie** eine Row ohne ihre ID: die `Prepared` ist nicht schreibbar, schreibbar ist nur die Row,
   und die nimmt die ID **stets als ersten Konstruktor-Parameter** (Domänenmodell-Regel „illegale
   Zustände unkonstruierbar" — eine Row mit leerer Platzhalter-ID ist unkonstruierbar). Dass `stage`
   die DTO **nicht** ein zweites Mal sieht, hält die Umdeutung an **einer** Stelle statt sie über
   zwei Trait-Methoden zu verschmieren. `tables()` listet **jede** Tabelle mit ihrem Statement,
   sodass der Preflight **jedes** probt (das erhält `auswerten`s heutige Eigenschaft *„hier terminal
   statt Commit-Retry"*). Der Ein-Tabellen-Fall ist schlicht eine `TableSpec`-Liste mit **einem**
   Element und kein Detail-Loop.

5. **`StoreUnavailable`-Form vereinheitlicht: redigierte `message` überall.** Heute trägt
   `obfuskierung_adapter::StoreUnavailable` **kein** Feld, die anderen drei ein redigiertes
   `message: String` — eine Drift. Alle vier tragen künftig `message: String`; `StoreSpec`
   fädelt den adaptereigenen Fehler über `type Error` + `fn unavailable(rusqlite::Error) ->
   Self::Error` durch. Die `message` trägt **ausschließlich** io-Zugriffs-Mechanik
   (rusqlite-Diagnose / statischer Grund), **nie** einen DTO-Feld- oder Zellwert
   ([`.rules/common/privacy-invarianten.md`](../../.rules/common/privacy-invarianten.md),
   [ADR-0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)) — geprüft durch
   den bestehenden `einlesen`-Leak-Test (`reason.contains(leck) == false`).

6. **Benennung:** Trait `StoreSpec` (spezifiziert die ganze Persistenz des Slice, nicht bloß
   „eine Row" — `auswerten` hat 5 Tabellen), Typen `SliceStore<S>`/`SliceUnit<S>`, `type Prepared`
   für die fehlbare, **id-lose** Vor-Umdeutung. Die SQL-fertige, id-tragende Row bleibt die
   **private** Struct des jeweiligen Slice (`LineRow`/`StagedEvaluation`), in `stage` mit der ID
   konstruiert — **kein** assoziierter Typ, damit „Row ohne ID" gar nicht erst benennbar ist.
   Code-Identifier **englisch** (Infrastruktur, nicht Domäne —
   [`.rules/common/sprache.md`](../../.rules/common/sprache.md)).

## Begründung

- **Ousterhout-tief statt DRY-clever.** Der generische Typ versteckt die Mechanik hinter einem
  schmalen Interface (`store_dto` + ein `StoreSpec`-`impl`), **ohne** die pro-Slice-Seam-Fläche
  zu verstecken — jeder Store zeigt weiterhin explizit, welchen Seam er erfüllt. Ein
  Alles-generierendes Makro hätte tiefer kollabiert, aber den größten Block ins Makro versteckt
  (schlechtere Fehlermeldungen, schlechtere Navigierbarkeit — gegen den „AI-navigable"-Wert der
  [CLAUDE.md](../../CLAUDE.md)). Eine bloße Helfer-Funktion hätte Struct/`new`/`UnitFactory`/
  Newtype **viermal** stehen lassen.
- **Typgetrieben.** `StoreSpec` als fixierte Trait-Grenze passt zu
  [`.rules/rust/compiler-getriebenes-implementieren.md`](../../.rules/rust/compiler-getriebenes-implementieren.md)
  („Traits und Typen zuerst"): der Compiler erzwingt, dass jeder Slice genau die fünf
  Bausteine liefert.
- **Eine Wahrheit gegen Drift.** Redaktions-Regel, Audit-Spalten, Preflight-Verhalten und der
  Mint-dann-Stage-Tanz leben **einmal** — die `obfuskierung`-Fehler-Drift kann so nicht wieder
  entstehen.
- **Seam-Vertrag unberührt.** `store_dto` ist `io`-intern; die vier `_adapter`-Seams behalten
  DTO **und** Rückgabe. [ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md) hält.

## Konsequenzen

- Die vier `store.rs` schrumpfen auf je **ein** `StoreSpec`-`impl` + eine dünne Seam-`impl`;
  `SliceStore`/`SliceUnit`/`StoreSpec` + der generische Outbox-Passthrough landen in
  `crates/io/src/sqlite/slice_store.rs` (entlastet das schon 946-zeilige `sqlite/mod.rs` —
  zahlt auf Kandidat 5 ein).
- `obfuskierung_adapter::StoreUnavailable` bekommt `message: String`; seine `_specs`
  (Black-Box) müssen den geänderten Konstruktor mitziehen — Implementierungs-Detail des
  Umsetzungs-Issues, kein Design-Blocker.
- **`.rules/sql` bleiben erzwungen:** DDL, INSERT und Indizes bleiben **pro Slice** authored
  (`&'static str`-Literale in der `TableSpec`-Liste) — der generische Store fasst **keine** DDL
  an und leitet **nichts** aus SQL ab; snake_case+Plural, `created_at`/`updated_at` (via
  `ensure_columns` über `TableSpec::name`), `CHECK` für enum-artige Spalten und indizierte
  FK-Spalten bleiben unverwässert.
- **Name, DDL und Insert einer Tabelle stehen beisammen** (`TableSpec`), statt als drei
  parallele Listen (`tables()`/`schema()`/`inserts()`) pro `impl` von Hand konsistent gehalten
  zu werden. Anlegen, Audit-Spalten-Nachrüstung und Preflight laufen über **dieselbe** Liste,
  also bekommt jede deklarierte Tabelle alle drei oder existiert nicht — die frühere Drift
  („in `schema()` angelegt, in `tables()` vergessen") ist nicht mehr darstellbar, ohne dass
  dafür SQL geparst werden müsste (das wäre die schlechtere Magie und gegen den
  „AI-navigable"-Wert — hier deklariert der Slice weiterhin selbst, es steht nur beisammen).
- **Rest-Drift bewusst intra-`impl`:** `TableSpec::insert` und `stage` bleiben zwei Sichten
  auf dieselben Statements — ein in `stage` gefahrenes, nirgends deklariertes Statement
  entginge dem Preflight und ließe die *„hier terminal statt Commit-Retry"*-Eigenschaft still
  auf einen Commit-Retry zurückfallen. Beide zeigen aber auf **dasselbe** `&'static str`-Literal,
  das neben seiner Tabelle steht. Das Netz ist der bestehende, **pro Slice handgeschriebene**
  Store-Test — insb. das Kriterium *„Preflight probt alle N Inserts"* des `auswerten`-Capstones
  ([Issue 0077](../issues/0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md)).
- Alle sechs Gates bleiben nach der Umsetzung grün; die vier bestehenden Store-Tests
  (feld-treue Ablage, terminal-redigierter Fehlschlag, Batch-Atomarität) bleiben die
  Spezifikation und sind der Regressionsschutz der Kollabierung.

## Verworfen, mit Grund

- **Deklaratives `slice_store!`-Makro** über das ganze Skelett — maximale Kollabierung, aber
  versteckt den größten Block im Makro (Fehlermeldungen/Navigierbarkeit) und wäre gegen den
  „AI-navigable"-Wert.
- **Nur Helfer-Funktion** (`stage_row`) — teilt nur den Tanz; Struct/`new`/`UnitFactory`/
  Newtype blieben viermal dupliziert.
- **Feldloses `StoreUnavailable` überall** — strukturell leak-frei und im Geist von „illegale
  Zustände unkonstruierbar", aber **ohne jede io-Diagnostik** beim Debuggen eines
  unerreichbaren Stores; verworfen zugunsten der etablierten Maserung des Codes (redigierte
  `message`, roher Fehler nur im `Debug` — wie `SqliteCommitError`) plus dem bestehenden
  Leak-Test als Netz.
- **`RETURNS_ID`-Assoc-Const / zwei Seams für die Rückgabe** — Typ-Apparat für eine
  Asymmetrie, die `map(drop)` schon trägt.

## Bewusst kein Glossar-Begriff

`SliceStore`/`StoreSpec` sind **`io`-Infrastruktur**, kein Domänenvokabular. `CONTEXT.md` ist
ein **reines Glossar frei von Implementierungsdetails** — allgemeine Programmier-/
Mechanik-Konzepte gehören dort **nicht** hinein (deshalb steht dort auch heute kein
„Ergebnis-Store"). Dieses ADR ist der kanonische Ort der Entscheidung; `CONTEXT.md` bleibt
unberührt.

## Bewusst vertagt

Die eigentliche Umsetzung (das `StoreSpec`-Trait, `SliceStore`/`SliceUnit`, die vier
`store.rs`-Umschreibungen, das `message`-Feld auf `obfuskierung`s `StoreUnavailable`) ist ein
**Folge-Issue** (GitHub + `docs/issues/`, [`.rules/common/tickets.md`](../../.rules/common/tickets.md)),
nicht dieses ADR.
