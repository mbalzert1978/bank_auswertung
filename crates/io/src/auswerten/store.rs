//! Der reale **auswerten-Ergebnis-Store** über `rusqlite`
//! ([Issue 0058](../../../../docs/issues/0058-auswerten-store-real-und-end-to-end.md)) — der
//! **letzte** slice-spezifische Store hinter dem öffentlichen [`EvaluationStoreSeam`], auf dem
//! transaktionalen Rückgrat aus
//! [Issue 0052](../../../../docs/issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md)
//! und **die Replikation des `fenstern`-Musters**
//! ([Issue 0057](../../../../docs/issues/0057-fenstern-store-real-und-end-to-end.md)). Er tritt an
//! die Stelle des `FakeDb`/`FakeUnit` der Fixtures; zusammen mit dem realen Lese-Seam
//! ([`AuswertungsfensterSource`](crate::auswerten::AuswertungsfensterSource)) läuft damit die **volle
//! Pipeline** `einlesen → obfuskieren → fenstern → auswerten` end-to-end **ohne einen einzigen Fake**
//! (bis zum `report`).
//!
//! **Atomarität (Tabelle = Wahrheit, [ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)):**
//! [`AuswertenSqliteStore`] ist die [`UnitFactory`](crate::UnitFactory) des Slice — je
//! Nachricht öffnet es eine [`AuswertenUnit`] über dem generischen [`SliceUnit`]. Deren
//! [`save`](EvaluationStoreSeam::save) **mintet die neue ID synchron an der DB** (`randomblob`) und
//! **stagt** die Ergebnis-`INSERT`s (Kopf + fünf Detail-Tabellen) in **dieselbe** committende
//! Transaktion wie der Ack. Diese Reichweite **publiziert keinen Zeiger** stromabwärts (sie ist das
//! Pipeline-Ende bis zum `report`), darum vergibt der Seam die neue ID nur DB-intern und gibt sie
//! **nicht** zurück; das Ergebnis referenziert sein Quell-Fenster **nur per `window_id`**
//! (Referenz-per-ID auf `auswertungsfenster.id`), nie eingebettet.
//!
//! **Sechs Tabellen (Kopf + fünf Detail-Listen):** `window_evaluations` trägt Fremdschlüssel, Währung
//! und Audit; die fünf Detail-Tabellen die **positionsgetreuen** Listen — Monats-Buckets,
//! Umbuchungen, Fingerabdrücke, eingeordnete Gegenkonten und die Orakel-Kategorien. Jede
//! Detail-Zeile referenziert den Kopf **per `evaluation_id`** (indiziert), **kein**
//! DB-`FOREIGN KEY`-Constraint (SQLite erzwingt FKs nur mit `PRAGMA foreign_keys`, das die synchrone
//! Outbox-Spine bewusst nicht schaltet).
//!
//! **Seit [Issue 0077](../../../../docs/issues/0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md)
//! trägt dieser Store nur noch das Slice-Spezifische** ([ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md)):
//! sein [`StoreSpec`]-`impl` (Schema + Inserts + Flatten + Row-mit-ID) und die dünne Seam-`impl`, die
//! [`SliceUnit::store_dto`](crate::sqlite::SliceUnit) aufruft. Store-Struct, `new`, Unit-Newtype,
//! Preflight/Minten, die [`UnitFactory`](crate::UnitFactory) und der Outbox-Passthrough leben
//! generisch in [`SliceStore`]/[`SliceUnit`]. Er ist der **Sechs-Tabellen-Fall** und damit der
//! Capstone der Abstraktion: **alle sechs** Tabellen stehen mit DDL und Ablage-Statement in **einer**
//! [`TableSpec`]-Liste (also legt `new` jede an, rüstet auf jeder die Audit-Spalten nach und der
//! Preflight probt **jedes** ihrer Statements), und [`stage`](StoreSpec::stage) liefert **eine**
//! Schreibung, die Kopf- und alle fünf Detail-Listen positionsgetreu in dieselbe Transaktion
//! schreibt. Der Seam-Vertrag Richtung Slice ist davon **unberührt**
//! ([ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)).
//!
//! **Fehlbar + terminal:** ein unerreichbarer Store (Schema fehlt) failt **synchron** in `save` als
//! [`StoreUnavailable`] (→ Domänen-`EvaluationError::StorageUnavailable`), terminal. Kein Dead-Letter,
//! kein Retry im Slice.

use auswerten_adapter::{
    CounterAccountRow, EvaluationStoreSeam, InternalTransferRow, MonthlyCashflowRow,
    StoreUnavailable, WindowEvaluationRow,
};
use rusqlite::{Transaction, params};

use crate::sqlite::{MintedId, SliceStore, SliceUnit, StagedRow, StoreSpec, TableSpec};

/// Die `auswerten`-Persistenz als [`StoreSpec`]: der **Sechs-Tabellen-Fall** — der Kopf
/// (`window_evaluations`) und seine fünf positionsgetreuen Detail-Listen, je mit ihrem `INSERT`, und
/// als fehlbare Umdeutung die u64→i64-Deutung der Monats-Beträge. Der Marker-Typ parametrisiert
/// [`SliceStore`]/[`SliceUnit`] auf diesen Slice.
pub struct AuswertenEvaluations;

impl StoreSpec for AuswertenEvaluations {
    type Dto = WindowEvaluationRow;
    /// Das abgeflachte Ergebnis ohne seine ID — schreibbar wird erst die `EvaluationRow`, die
    /// `stage` daraus **mit** der geminteten ID baut (Kopf **und** alle fünf Detail-Listen tragen sie).
    type Prepared = EvaluationFields;
    type Error = StoreUnavailable;

    fn tables() -> &'static [TableSpec] {
        TABLES
    }

    fn flatten(evaluation: &WindowEvaluationRow) -> Result<EvaluationFields, StoreUnavailable> {
        EvaluationFields::try_from(evaluation)
    }

    /// Baut **eine** Schreibung über alle sechs Tabellen: erst die Kopf-Zeile unter der frisch
    /// geminteten `id`, dann positionsgetreu die fünf Detail-Listen unter derselben `id`.
    fn stage(fields: EvaluationFields, id: MintedId) -> StagedRow {
        let row = EvaluationRow::new(id, fields);
        Box::new(move |tx| row.insert(tx))
    }

    /// Reicht die `rusqlite`-Diagnose eines misslungenen Store-Zugriffs durch (fehlendes Schema,
    /// DB-Fehler) — Mechanik, nie ein Zell-/Feldwert.
    fn unavailable(error: rusqlite::Error) -> StoreUnavailable {
        store_unavailable(&error.to_string())
    }
}

/// Der reale `auswerten`-Ergebnis-Store: der generische [`SliceStore`] über [`AuswertenEvaluations`]
/// und damit die [`UnitFactory`](crate::UnitFactory) des Slice. `Clone` teilt dasselbe Rückgrat
/// (`Rc` im Innern).
pub type AuswertenSqliteStore = SliceStore<AuswertenEvaluations>;

/// Eine `auswerten`-Arbeitseinheit: der gestagete Ergebnis-Speicher (`EvaluationStoreSeam`) **und**
/// die Outbox (`OutboxPublisher`) in einem, atomar committbar (`UnitOfWork`). Dass der Publish-Rand
/// dieser Reichweite ungenutzt bleibt, ändert die Einheit nicht: sie umspannt Ack und Ablage und
/// wächst ohne Umbau um den späteren `report`-Anstoß.
pub type AuswertenUnit = SliceUnit<AuswertenEvaluations>;

impl EvaluationStoreSeam for AuswertenUnit {
    /// Mintet die neue ID **synchron an der DB** (`randomblob`) und stagt Kopf- und alle fünf
    /// Detail-`INSERT`s in die committende Transaktion; ein unerreichbarer Store (Schema fehlt /
    /// DB-Fehler) failt **hier** terminal als redigierte [`StoreUnavailable`]. Der Preflight probt
    /// **alle sechs** Ablage-Statements (Kopf **und** die fünf Detail — alle stehen in `TABLES`):
    /// fehlt auch nur eine Detail-Tabelle, failt es hier terminal statt erst als Commit-Retry.
    ///
    /// Diese Reichweite ist das **Pipeline-Ende** und publiziert nichts, darum verwirft sie die
    /// gemintete ID per `map(drop)`: die ID bleibt **DB-vergeben** und bindet die Detail-Zeilen an
    /// ihren Kopf, sie reist nur nirgends als Zeiger weiter
    /// ([ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)).
    fn save(&self, evaluation: &WindowEvaluationRow) -> Result<(), StoreUnavailable> {
        self.store_dto(evaluation).map(drop)
    }
}

/// Eine redigierte [`StoreUnavailable`] aus io-eigener Diagnose (Zugriffs-Mechanik, nie ein
/// Zell-/Feldwert — der Seam-Vertrag verlangt die Redaktion).
fn store_unavailable(message: &str) -> StoreUnavailable {
    StoreUnavailable {
        message: format!("window_evaluations-Store nicht erreichbar: {message}"),
    }
}

/// Deutet einen **vorzeichenlosen** Betrag (Cashflow-Bucket) in die `i64`-Spalte. Eine Magnitude
/// jenseits `i64::MAX` ist unrealistisch und failt terminal als [`StoreUnavailable`]
/// (Speicher-Mechanik, nie ein Rohwert) — dieselbe Linie, die `obfuskierung` beim Betrag zieht.
fn magnitude(value: u64) -> Result<i64, StoreUnavailable> {
    i64::try_from(value).map_err(|_overflow| {
        store_unavailable("Cashflow-Betrag jenseits des darstellbaren Bereichs")
    })
}

/// Die abgeflachte [`WindowEvaluationRow`] in ihrer besitzenden Speicher-Form — **ohne** ihre ID und
/// darum **nicht** schreibbar. Der einzige Ort, an dem dieser Store die Adapter-DTO anfasst; alles
/// Fehlbare (die u64→i64-Deutung der Monats-Beträge) ist danach entschieden. Schreibbar wird erst die
/// [`EvaluationRow`], die [`stage`](StoreSpec::stage) daraus **mit** der geminteten ID baut.
///
/// `pub` aus Sichtbarkeits-Zwang (sie steht als `Prepared` in der Signatur eines öffentlichen
/// Trait-`impl`, `private_interfaces`), **nicht** als API: das Modul `store` ist privat, also bleibt
/// sie außerhalb von `io` nicht konstruierbar und nicht lesbar (benennbar ist sie als
/// `<AuswertenEvaluations as StoreSpec>::Prepared` schon — nur eben inert).
pub struct EvaluationFields {
    window_id: String,
    currency: String,
    months: Vec<MonthRow>,
    transfers: Vec<TransferRow>,
    fingerprints: Vec<String>,
    counter_accounts: Vec<CounterRow>,
    categories: Vec<CategoryRow>,
}

impl TryFrom<&WindowEvaluationRow> for EvaluationFields {
    type Error = StoreUnavailable;

    /// Übernimmt die fünf Detail-Listen in ihrer **Reihenfolge** — die *ist* die Position; die
    /// Positionszahl entsteht erst beim Schreiben ([`insert_positional`]), kann der Reihenfolge also
    /// nicht widersprechen. Fehlbar allein in der u64→i64-Deutung der Monats-Beträge, die damit
    /// **vor** dem Minten läuft, sodass [`EvaluationRow::new`] die schreibbare Row **in einem Zug
    /// mit** der ID bauen kann.
    fn try_from(evaluation: &WindowEvaluationRow) -> Result<Self, StoreUnavailable> {
        Ok(Self {
            window_id: evaluation.window_id.clone(),
            currency: evaluation.currency.clone(),
            months: evaluation
                .months
                .iter()
                .map(MonthRow::try_from)
                .collect::<Result<_, _>>()?,
            transfers: evaluation
                .transfers
                .iter()
                .map(TransferRow::try_from)
                .collect::<Result<_, _>>()?,
            fingerprints: evaluation.fingerprints.clone(),
            counter_accounts: evaluation
                .counter_accounts
                .iter()
                .map(CounterRow::from)
                .collect(),
            categories: evaluation
                .categories
                .iter()
                .map(CategoryRow::from)
                .collect(),
        })
    }
}

/// Ein Monats-Bucket in seiner Speicher-Form: die Cashflow-Beträge schon als `i64` vorgeprüft.
struct MonthRow {
    year: i64,
    month: i64,
    incoming_minor_units: i64,
    outgoing_minor_units: i64,
}

/// Eine **Umbuchung** in ihrer Speicher-Form — derselbe Monats-Bucket wie [`MonthRow`], zusätzlich
/// mit der Rolle des Gegenkontos, die sie von den übrigen trennt.
struct TransferRow {
    year: i64,
    month: i64,
    role: String,
    incoming_minor_units: i64,
    outgoing_minor_units: i64,
}

/// Ein eingeordnetes Gegenkonto in seiner Speicher-Form: `kind` die `CHECK`-erzwungene Diskriminante,
/// `role` nur beim eigenen Konto gesetzt (der Summentyp bleibt so auch flach unkonstruierbar-illegal).
struct CounterRow {
    kind: &'static str,
    role: Option<String>,
}

/// Eine zugeordnete Kategorie in ihrer Speicher-Form.
struct CategoryRow {
    fingerprint: String,
    category: String,
}

impl TryFrom<&MonthlyCashflowRow> for MonthRow {
    type Error = StoreUnavailable;

    fn try_from(cashflow: &MonthlyCashflowRow) -> Result<Self, StoreUnavailable> {
        Ok(Self {
            year: cashflow.year,
            month: cashflow.month,
            incoming_minor_units: magnitude(cashflow.incoming_minor_units)?,
            outgoing_minor_units: magnitude(cashflow.outgoing_minor_units)?,
        })
    }
}

impl TryFrom<&InternalTransferRow> for TransferRow {
    type Error = StoreUnavailable;

    fn try_from(transfer: &InternalTransferRow) -> Result<Self, StoreUnavailable> {
        Ok(Self {
            year: transfer.year,
            month: transfer.month,
            role: transfer.role.clone(),
            incoming_minor_units: magnitude(transfer.incoming_minor_units)?,
            outgoing_minor_units: magnitude(transfer.outgoing_minor_units)?,
        })
    }
}

impl From<&CounterAccountRow> for CounterRow {
    fn from(counter_account: &CounterAccountRow) -> Self {
        match counter_account {
            CounterAccountRow::Own { role } => Self {
                kind: "own",
                role: Some(role.clone()),
            },
            CounterAccountRow::Foreign => Self {
                kind: "foreign",
                role: None,
            },
        }
    }
}

impl From<&auswerten_adapter::CategoryRow> for CategoryRow {
    fn from(category: &auswerten_adapter::CategoryRow) -> Self {
        Self {
            fingerprint: category.fingerprint.clone(),
            category: category.category.clone(),
        }
    }
}

/// Die SQL-fertigen Zeilen **aller fünf** Tabellen: die abgeflachten Felder **unter** ihrer
/// DB-vergebenen ID, die Kopf und alle fünf Detail-Listen gemeinsam tragen. Sie reist in die
/// gestagete Ablage-Closure und wird erst am Commit-Rand geschrieben. Die ID ist erster Parameter von
/// [`new`](EvaluationRow::new); dass es die eben an der DB gemintete ist, trägt ihr Typ
/// [`MintedId`] — eine von Hand gebaute Schein-ID, die Kopf und die fünf Detail-Listen unter einem
/// Schein-Primärschlüssel zusammenbände, ist hier ein Compile-Fehler.
struct EvaluationRow {
    id: MintedId,
    fields: EvaluationFields,
}

impl EvaluationRow {
    /// Baut die schreibbare Row unter der frisch geminteten DB-`id`. Diese Reichweite publiziert
    /// keinen Zeiger — die ID bleibt DB-intern und bindet nur die Detail-Zeilen an ihren Kopf.
    const fn new(id: MintedId, fields: EvaluationFields) -> Self {
        Self { id, fields }
    }

    /// Schreibt Kopf- und Detail-Zeilen in die committende Transaktion. Jede der fünf Detail-Listen
    /// läuft **positionsgetreu** über [`insert_positional`], das die `enumerate`→`position`-Nummerierung
    /// an **einer** Stelle bündelt; jede Closure trägt nur noch das listen-eigene Statement und seine
    /// Feld-Params.
    fn insert(&self, tx: &Transaction<'_>) -> rusqlite::Result<()> {
        let fields = &self.fields;
        tx.execute(
            INSERT_EVALUATION,
            params![self.id, fields.window_id, fields.currency],
        )?;
        insert_positional(&fields.months, |position, month| {
            tx.execute(
                INSERT_MONTH,
                params![
                    self.id,
                    position,
                    month.year,
                    month.month,
                    month.incoming_minor_units,
                    month.outgoing_minor_units,
                ],
            )
            .map(drop)
        })?;
        insert_positional(&fields.transfers, |position, transfer| {
            tx.execute(
                INSERT_TRANSFER,
                params![
                    self.id,
                    position,
                    transfer.year,
                    transfer.month,
                    transfer.role,
                    transfer.incoming_minor_units,
                    transfer.outgoing_minor_units,
                ],
            )
            .map(drop)
        })?;
        insert_positional(&fields.fingerprints, |position, fingerprint| {
            tx.execute(INSERT_FINGERPRINT, params![self.id, position, fingerprint])
                .map(drop)
        })?;
        insert_positional(&fields.counter_accounts, |position, counter_account| {
            tx.execute(
                INSERT_COUNTER_ACCOUNT,
                params![
                    self.id,
                    position,
                    counter_account.kind,
                    counter_account.role
                ],
            )
            .map(drop)
        })?;
        insert_positional(&fields.categories, |position, category| {
            tx.execute(
                INSERT_CATEGORY,
                params![self.id, position, category.fingerprint, category.category],
            )
            .map(drop)
        })
    }
}

/// Schreibt eine Detail-Liste **positionsgetreu**: nummeriert jede Zeile (`enumerate` → `position`
/// über [`position_or_sql_error`]) und überlässt das eigentliche `INSERT` der Closure, die Statement
/// und Feld-Params kennt. Bündelt die `enumerate`/Positions-Mechanik der fünf Detail-Listen an **einer**
/// Stelle, sodass je Liste nur ihr Statement und ihre Felder übrig bleiben.
fn insert_positional<T>(
    items: &[T],
    mut write: impl FnMut(i64, &T) -> rusqlite::Result<()>,
) -> rusqlite::Result<()> {
    items
        .iter()
        .enumerate()
        .try_for_each(|(index, item)| write(position_or_sql_error(index)?, item))
}

/// Wandelt einen Index in die `position`-Spalte **innerhalb** der Schreib-Closure — dort ist der
/// Rückgabetyp `rusqlite::Result`, darum wird der Überlauf in einen `rusqlite`-Fehler gehüllt (er ist
/// so unrealistisch wie der in [`position`]; die Redaktion trägt keinen Rohwert).
fn position_or_sql_error(index: usize) -> rusqlite::Result<i64> {
    i64::try_from(index).map_err(|_overflow| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
            "Ergebnis-Liste trägt mehr Einträge als sich ordnen lassen",
        )))
    })
}

/// Die **fünf** Tabellen des Slice, je mit ihrem DDL und ihrem Ablage-`INSERT` — der Capstone-Fall
/// der Abstraktion. Anlegen, Audit-Spalten-Nachrüstung und Preflight laufen alle über diese Liste,
/// also gilt der Preflight-Beweis für **jede**: eine Tabelle, die eines davon verpasst, ist gar nicht
/// erst deklarierbar.
const TABLES: &[TableSpec] = &[
    TableSpec {
        name: "window_evaluations",
        ddl: SCHEMA_EVALUATIONS,
        insert: INSERT_EVALUATION,
    },
    TableSpec {
        name: "window_evaluation_months",
        ddl: SCHEMA_MONTHS,
        insert: INSERT_MONTH,
    },
    TableSpec {
        name: "window_evaluation_transfers",
        ddl: SCHEMA_TRANSFERS,
        insert: INSERT_TRANSFER,
    },
    TableSpec {
        name: "window_evaluation_fingerprints",
        ddl: SCHEMA_FINGERPRINTS,
        insert: INSERT_FINGERPRINT,
    },
    TableSpec {
        name: "window_evaluation_counter_accounts",
        ddl: SCHEMA_COUNTER_ACCOUNTS,
        insert: INSERT_COUNTER_ACCOUNT,
    },
    TableSpec {
        name: "window_evaluation_categories",
        ddl: SCHEMA_CATEGORIES,
        insert: INSERT_CATEGORY,
    },
];

/// Das Schema der **Kopf**-Tabelle: `window_evaluations` trägt den **Fremdschlüssel** `window_id` auf
/// `auswertungsfenster` (indiziert) und die Währung unter der DB-vergebenen ID.
/// `created_at`/`updated_at` (Epoch-Millis) tragen den Audit-Zeitstempel — die Tabelle ist
/// append-only, `updated_at` fällt mit `created_at` zusammen
/// (`.rules/sql/schema-konventionen.md`).
const SCHEMA_EVALUATIONS: &str = "\
CREATE TABLE IF NOT EXISTS window_evaluations (
    id         TEXT    PRIMARY KEY,
    window_id  TEXT    NOT NULL,
    currency   TEXT    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
);
CREATE INDEX IF NOT EXISTS window_evaluations_window_id_idx
    ON window_evaluations (window_id);";

/// Das Schema der **Monats-Bucket**-Detail-Tabelle: die positionsgetreuen Cashflow-Buckets, per
/// `evaluation_id` (indiziert) an ihren Kopf gebunden — **kein** DB-`FOREIGN KEY`-Constraint (SQLite
/// erzwingt FKs nur mit aktiviertem `PRAGMA foreign_keys`, das die synchrone Outbox-Spine bewusst
/// nicht schaltet). Audit-Zeitstempel wie bei der Kopf-Tabelle; das gilt für alle fünf Detail-Tabellen.
const SCHEMA_MONTHS: &str = "\
CREATE TABLE IF NOT EXISTS window_evaluation_months (
    evaluation_id        TEXT    NOT NULL,
    position             INTEGER NOT NULL,
    year                 INTEGER NOT NULL,
    month                INTEGER NOT NULL,
    incoming_minor_units INTEGER NOT NULL,
    outgoing_minor_units INTEGER NOT NULL,
    created_at           INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at           INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    PRIMARY KEY (evaluation_id, position)
);
CREATE INDEX IF NOT EXISTS window_evaluation_months_evaluation_id_idx
    ON window_evaluation_months (evaluation_id);";

/// Das Schema der **Umbuchungs**-Detail-Tabelle: die positionsgetreuen Monats-Buckets der Umbuchungen
/// zwischen eigenen Konten — das, was aus `window_evaluation_months` herausgenettet wurde (Issue
/// 0004). Sie steht **neben** den Monats-Buckets statt in ihnen, weil beide Sichten sonst nicht mehr
/// getrennt lesbar wären; der Brutto-Fluss eines Monats ist die Summe aus beiden Tabellen.
///
/// `role` ist per `CHECK` auf das Vier-Wort-Vokabular der Konto-Rollen erzwungen
/// (`.rules/sql/schema-konventionen.md`) — anders als bei den Gegenkonten, wo `role` optional ist und
/// stattdessen an `kind` gekoppelt wird: eine Umbuchung **ohne** Rolle gibt es nicht.
const SCHEMA_TRANSFERS: &str = "\
CREATE TABLE IF NOT EXISTS window_evaluation_transfers (
    evaluation_id        TEXT    NOT NULL,
    position             INTEGER NOT NULL,
    year                 INTEGER NOT NULL,
    month                INTEGER NOT NULL,
    role                 TEXT    NOT NULL CHECK (role IN ('giro', 'haushalt', 'spar', 'vorlauf')),
    incoming_minor_units INTEGER NOT NULL,
    outgoing_minor_units INTEGER NOT NULL,
    created_at           INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at           INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    PRIMARY KEY (evaluation_id, position)
);
CREATE INDEX IF NOT EXISTS window_evaluation_transfers_evaluation_id_idx
    ON window_evaluation_transfers (evaluation_id);";

/// Das Schema der **Fingerabdruck**-Detail-Tabelle: die positionsgetreuen, im Fenster vorkommenden
/// Fingerabdrücke.
const SCHEMA_FINGERPRINTS: &str = "\
CREATE TABLE IF NOT EXISTS window_evaluation_fingerprints (
    evaluation_id TEXT    NOT NULL,
    position      INTEGER NOT NULL,
    fingerprint   TEXT    NOT NULL,
    created_at    INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at    INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    PRIMARY KEY (evaluation_id, position)
);
CREATE INDEX IF NOT EXISTS window_evaluation_fingerprints_evaluation_id_idx
    ON window_evaluation_fingerprints (evaluation_id);";

/// Das Schema der **Gegenkonten**-Detail-Tabelle: die positionsgetreu eingeordneten Gegenkonten.
/// `kind` ist per `CHECK` auf `own`/`foreign` erzwungen; ein **zweites `CHECK`** koppelt `role` an
/// `kind` (nur `own` trägt eine Rolle, `foreign` nie) — so ist der Summentyp `CounterAccountRow` auch
/// **auf DB-Ebene** unkonstruierbar-illegal statt „geprüft" (ADR-0005), nicht nur im Rust-Modell.
const SCHEMA_COUNTER_ACCOUNTS: &str = "\
CREATE TABLE IF NOT EXISTS window_evaluation_counter_accounts (
    evaluation_id TEXT    NOT NULL,
    position      INTEGER NOT NULL,
    kind          TEXT    NOT NULL CHECK (kind IN ('own', 'foreign')),
    role          TEXT,
    created_at    INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at    INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    PRIMARY KEY (evaluation_id, position),
    CHECK ((kind = 'own' AND role IS NOT NULL) OR (kind = 'foreign' AND role IS NULL))
);
CREATE INDEX IF NOT EXISTS window_evaluation_counter_accounts_evaluation_id_idx
    ON window_evaluation_counter_accounts (evaluation_id);";

/// Das Schema der **Kategorien**-Detail-Tabelle: die positionsgetreu zugeordneten Orakel-Kategorien,
/// je unter dem Fingerabdruck, auf den sie antworten.
const SCHEMA_CATEGORIES: &str = "\
CREATE TABLE IF NOT EXISTS window_evaluation_categories (
    evaluation_id TEXT    NOT NULL,
    position      INTEGER NOT NULL,
    fingerprint   TEXT    NOT NULL,
    category      TEXT    NOT NULL,
    created_at    INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at    INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    PRIMARY KEY (evaluation_id, position)
);
CREATE INDEX IF NOT EXISTS window_evaluation_categories_evaluation_id_idx
    ON window_evaluation_categories (evaluation_id);";

const INSERT_EVALUATION: &str = "\
INSERT INTO window_evaluations (id, window_id, currency, created_at, updated_at)
VALUES (?1, ?2, ?3,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

const INSERT_MONTH: &str = "\
INSERT INTO window_evaluation_months
    (evaluation_id, position, year, month, incoming_minor_units, outgoing_minor_units,
     created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

const INSERT_TRANSFER: &str = "\
INSERT INTO window_evaluation_transfers
    (evaluation_id, position, year, month, role, incoming_minor_units, outgoing_minor_units,
     created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

const INSERT_FINGERPRINT: &str = "\
INSERT INTO window_evaluation_fingerprints (evaluation_id, position, fingerprint, created_at, updated_at)
VALUES (?1, ?2, ?3,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

const INSERT_COUNTER_ACCOUNT: &str = "\
INSERT INTO window_evaluation_counter_accounts (evaluation_id, position, kind, role, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

const INSERT_CATEGORY: &str = "\
INSERT INTO window_evaluation_categories (evaluation_id, position, fingerprint, category, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::num::NonZeroU32;
    use std::rc::Rc;

    use application_kernel::PublishCommand;
    use auswerten_adapter::{
        BookingDirection, CategorizerSeam, CategorizerUnavailable, CategoryAnswerRow,
        CategoryRegistrySeam, CategoryRequestRow, OccurrenceRow, OwnAccountRow, OwnAccountsConfig,
        RegistryUnavailable,
    };
    use obfuskierung_adapter::ObfuscatedMessage;
    use rusqlite::params;

    use super::*;
    use crate::auswerten::{
        AuswertungsfensterSource, CONSUMED_HISTORY_MESSAGE_TYPE,
        CONSUMED_MESSAGE_TYPE as AUSWERTEN_CONSUMED, OccurrenceSink,
        consumer as auswerten_consumer, history_consumer,
    };
    use crate::fenstern::{
        CONSUMED_MESSAGE_TYPE as FENSTERN_CONSUMED, FensternSqliteStore, ObfuskiertWindowSource,
        START_MESSAGE_TYPE, consumer as fenstern_consumer, start_consumer,
    };
    use crate::obfuskierung::seed_marker;
    use crate::outbox::UnitOfWork;
    use crate::pipeline_test_support::{
        CapturingAck, SEED, Stage, drain, register_fresh, seed_through, stamp,
    };
    use crate::sqlite::SqliteOutbox;
    use crate::{Dispatcher, MessageId, UnitFactory};

    const OBFUSCATED_MESSAGE_TYPE: &str = <ObfuscatedMessage as PublishCommand>::MESSAGE_TYPE;

    /// Der Fingerabdruck aller vier Buchungen der `mehrseitig.csv`: die Gegenpartei ist eine Firma
    /// mit fester Gläubiger-ID (kein Durchleiter), also teilen sie **einen** Fingerabdruck.
    const COMPANY_FINGERPRINT: &str = "COMPANY:DE98ZZZ09999999999";

    /// Ein Register-Doppel, das **alles** kennt: es gibt jeden Kandidaten als bekannt zurück, sodass
    /// nie ein Orakel befragt wird und die Kategorien-Liste jedes Fensters leer bleibt. So ist der
    /// e2e-Test unabhängig von einem Orakel-Ausgang (das Register selbst ist Issue 0009, das Orakel
    /// Issue 0011 — beide bleiben hier bewusst Test-Doppel).
    struct AllKnownRegistry;
    impl CategoryRegistrySeam for AllKnownRegistry {
        fn known_among(&self, candidates: &[String]) -> Result<Vec<String>, RegistryUnavailable> {
            Ok(candidates.to_vec())
        }
    }

    /// Ein Orakel-Doppel, das nie etwas beiträgt (es wird ohnehin nicht befragt, s. [`AllKnownRegistry`]).
    struct NoopCategorizer;
    impl CategorizerSeam for NoopCategorizer {
        fn ask(
            &self,
            _requests: &[CategoryRequestRow],
        ) -> Result<Vec<CategoryAnswerRow>, CategorizerUnavailable> {
            Ok(Vec::new())
        }
    }

    /// Der Abnehmer der fenster-übergreifenden Reichweite: sammelt jede Lieferung (der reale ist der
    /// Wiederkehr-Analyzer, Issue 0005). `Clone` teilt denselben Puffer.
    #[derive(Clone, Default)]
    struct CollectingSink {
        deliveries: Rc<RefCell<Vec<Vec<OccurrenceRow>>>>,
    }
    impl OccurrenceSink for CollectingSink {
        fn receive(&self, occurrences: Vec<OccurrenceRow>) {
            self.deliveries.borrow_mut().push(occurrences);
        }
    }

    /// Die token-orientierte Eigene-Konten-Config für den Lauf: der Marker aus dem Test-Seed und das
    /// **reale** Gegenkonto-Token der Fixture (aus `obfuskiert` gelesen, nicht hartkodiert) als eigenes
    /// Giro-Konto — so lässt sich die token-basierte `Eigen`-Auflösung end-to-end zeigen.
    fn own_accounts_config(outbox: &SqliteOutbox) -> rusqlite::Result<OwnAccountsConfig> {
        let token: String = outbox.with_connection(|conn| {
            conn.query_row(
                "SELECT DISTINCT counter_account FROM obfuskiert WHERE counter_account IS NOT NULL;",
                [],
                |row| row.get(0),
            )
        })?;
        Ok(OwnAccountsConfig {
            seed_marker: seed_marker(SEED),
            accounts: vec![OwnAccountRow {
                token,
                role: "giro".to_string(),
            }],
        })
    }

    /// Ein zurückgelesenes Fenster-Ergebnis (nur für Tests): Währung, Monats-Buckets, Fingerabdrücke,
    /// eingeordnete Gegenkonten (`kind`, optionale Rolle) und Kategorien.
    #[derive(Debug, PartialEq, Eq)]
    struct StoredEvaluation {
        currency: String,
        months: Vec<(i64, i64, i64, i64)>,
        transfers: Vec<StoredTransfer>,
        fingerprints: Vec<String>,
        counter_accounts: Vec<(String, Option<String>)>,
        categories: Vec<(String, String)>,
    }

    /// Liest die abgelegten Ergebnisse zurück, in Ablage-Reihenfolge (`rowid`), je mit ihren
    /// positionsgetreuen Detail-Listen. Prüft nebenbei, dass jede `window_id` auf ein reales
    /// Auswertungsfenster zeigt (Referenz-per-ID, ADR-0012).
    fn stored_evaluations(outbox: &SqliteOutbox) -> rusqlite::Result<Vec<StoredEvaluation>> {
        outbox.with_connection(|conn| {
            let mut heads = conn.prepare(
                "SELECT e.id, e.currency FROM window_evaluations e \
                 JOIN auswertungsfenster w ON w.id = e.window_id ORDER BY e.rowid;",
            )?;
            let ids = heads
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            ids.into_iter()
                .map(|(id, currency)| {
                    Ok(StoredEvaluation {
                        currency,
                        months: read_months(conn, &id)?,
                        transfers: read_transfers(conn, &id)?,
                        fingerprints: read_fingerprints(conn, &id)?,
                        counter_accounts: read_counter_accounts(conn, &id)?,
                        categories: read_categories(conn, &id)?,
                    })
                })
                .collect()
        })
    }

    fn read_months(
        conn: &rusqlite::Connection,
        id: &str,
    ) -> rusqlite::Result<Vec<(i64, i64, i64, i64)>> {
        conn.prepare(
            "SELECT year, month, incoming_minor_units, outgoing_minor_units \
             FROM window_evaluation_months WHERE evaluation_id = ?1 ORDER BY position;",
        )?
        .query_map(params![id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect()
    }

    /// Eine zurückgelesene Umbuchungs-Zeile: Jahr, Monat, Rolle, Zu-, Abfluss.
    type StoredTransfer = (i64, i64, String, i64, i64);

    fn read_transfers(
        conn: &rusqlite::Connection,
        id: &str,
    ) -> rusqlite::Result<Vec<StoredTransfer>> {
        conn.prepare(
            "SELECT year, month, role, incoming_minor_units, outgoing_minor_units \
             FROM window_evaluation_transfers WHERE evaluation_id = ?1 ORDER BY position;",
        )?
        .query_map(params![id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?
        .collect()
    }

    fn read_fingerprints(conn: &rusqlite::Connection, id: &str) -> rusqlite::Result<Vec<String>> {
        conn.prepare(
            "SELECT fingerprint FROM window_evaluation_fingerprints \
             WHERE evaluation_id = ?1 ORDER BY position;",
        )?
        .query_map(params![id], |row| row.get(0))?
        .collect()
    }

    fn read_counter_accounts(
        conn: &rusqlite::Connection,
        id: &str,
    ) -> rusqlite::Result<Vec<(String, Option<String>)>> {
        conn.prepare(
            "SELECT kind, role FROM window_evaluation_counter_accounts \
             WHERE evaluation_id = ?1 ORDER BY position;",
        )?
        .query_map(params![id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect()
    }

    fn read_categories(
        conn: &rusqlite::Connection,
        id: &str,
    ) -> rusqlite::Result<Vec<(String, String)>> {
        conn.prepare(
            "SELECT fingerprint, category FROM window_evaluation_categories \
             WHERE evaluation_id = ?1 ORDER BY position;",
        )?
        .query_map(params![id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect()
    }

    fn occurrence(day: i64, amount_minor_units: u64) -> OccurrenceRow {
        OccurrenceRow {
            fingerprint: COMPANY_FINGERPRINT.to_string(),
            booking_day: day,
            amount_minor_units,
            direction: BookingDirection::Outgoing,
        }
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn die_volle_pipeline_legt_beide_reichweiten_real_ab() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        seed_through(&outbox, Stage::Obfuskierung, "mehrseitig.csv")?;

        let window_source = ObfuskiertWindowSource::new(outbox.clone());
        let fenstern_store = FensternSqliteStore::new(outbox.clone())?;
        let window_days = NonZeroU32::new(3).expect("Seitengröße ist nicht null");

        let auswerten_source = AuswertungsfensterSource::new(outbox.clone());
        let auswerten_store = AuswertenSqliteStore::new(outbox.clone())?;
        let registry = AllKnownRegistry;
        let categorizer = NoopCategorizer;
        let sink = CollectingSink::default();
        let own_accounts = own_accounts_config(&outbox)?;

        let mut dispatcher = Dispatcher::new();
        register_fresh(
            &mut dispatcher,
            OBFUSCATED_MESSAGE_TYPE,
            Box::new(CapturingAck::new(outbox.clone())),
        );
        register_fresh(
            &mut dispatcher,
            START_MESSAGE_TYPE,
            Box::new(start_consumer(&window_source, fenstern_store.clone())),
        );
        register_fresh(
            &mut dispatcher,
            FENSTERN_CONSUMED,
            Box::new(fenstern_consumer(
                window_days,
                &window_source,
                fenstern_store,
            )),
        );
        register_fresh(
            &mut dispatcher,
            AUSWERTEN_CONSUMED,
            Box::new(auswerten_consumer(
                &auswerten_source,
                &registry,
                &categorizer,
                own_accounts,
                auswerten_store.clone(),
            )),
        );
        register_fresh(
            &mut dispatcher,
            CONSUMED_HISTORY_MESSAGE_TYPE,
            Box::new(history_consumer(
                &auswerten_source,
                sink.clone(),
                auswerten_store,
            )),
        );

        outbox.enqueue(START_MESSAGE_TYPE, "")?;
        let outcomes = drain(&outbox, &dispatcher)?;

        assert!(
            outcomes.iter().all(|(_, outcome)| outcome.is_ok()),
            "die volle Pipeline muss ohne Fehler durchlaufen: {outcomes:?}"
        );

        // Pro-Fenster-Reichweite: drei Fenster, jedes mit dem einen Fingerabdruck, dem als eigen
        // (giro) aufgelösten Gegenkonto und ohne Kategorien.
        //
        // **Der Cashflow ist leer, und das ist das Ergebnis.** Die vier Buchungen der Fixture zeigen
        // alle auf dasselbe Gegenkonto, und dieses ist als eigenes Giro-Konto konfiguriert — also
        // besteht dieser Lauf **ausschließlich** aus Umbuchungen und keine einzige davon ist ein
        // Abfluss (Issue 0004). Verloren ist trotzdem nichts: dieselben Beträge stehen, real in der
        // eigenen Tabelle abgelegt, in der Umbuchungs-Sicht.
        let own_giro = ("own".to_string(), Some("giro".to_string()));
        let giro = |amount| (2024, 6, "giro".to_string(), 0, amount);
        assert_eq!(
            stored_evaluations(&outbox)?,
            vec![
                StoredEvaluation {
                    currency: "EUR".to_string(),
                    months: vec![],
                    transfers: vec![giro(3000)],
                    fingerprints: vec![COMPANY_FINGERPRINT.to_string()],
                    counter_accounts: vec![own_giro.clone()],
                    categories: vec![],
                },
                StoredEvaluation {
                    currency: "EUR".to_string(),
                    months: vec![],
                    transfers: vec![giro(3000)],
                    fingerprints: vec![COMPANY_FINGERPRINT.to_string()],
                    counter_accounts: vec![own_giro.clone()],
                    categories: vec![],
                },
                StoredEvaluation {
                    currency: "EUR".to_string(),
                    months: vec![],
                    transfers: vec![giro(4000)],
                    fingerprints: vec![COMPANY_FINGERPRINT.to_string()],
                    counter_accounts: vec![own_giro],
                    categories: vec![],
                },
            ],
            "drei Fenster-Ergebnisse, jedes real abgelegt"
        );

        // Fenster-übergreifende Reichweite: **eine** Lieferung, die Vorkommen aller vier Buchungen
        // chronologisch unter demselben Fingerabdruck — das, was die pro-Fenster-Sicht nicht sehen kann.
        assert_eq!(
            *sink.deliveries.borrow(),
            vec![vec![
                occurrence(stamp(2024, 6, 10), 4000),
                occurrence(stamp(2024, 6, 15), 3000),
                occurrence(stamp(2024, 6, 19), 2000),
                occurrence(stamp(2024, 6, 20), 1000),
            ]],
            "genau eine fenster-übergreifende Lieferung über die ganze Zeitachse"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ein_unerreichbarer_store_failt_save_terminal_und_redigiert() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        // Bewusst an `new` vorbeigebaut: die Ergebnis-Tabellen fehlen → unerreichbar.
        let store = AuswertenSqliteStore::without_schema(outbox.clone());
        let unit = store.begin(MessageId("egal"));

        let fehler = unit
            .save(&leeres_ergebnis())
            .expect_err("ein fehlendes Schema muss save terminal failen lassen");
        assert!(
            !format!("{fehler}").is_empty(),
            "der terminale Store-Fehler trägt eine (redigierte) Diagnose"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        clippy::unwrap_in_result,
        reason = "Test-Helfer: assert_eq! und expect sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn kopf_und_alle_fuenf_detail_listen_landen_in_ihren_tabellen() -> rusqlite::Result<()> {
        // Der Lauf oben zeigt die Ablage im Zusammenspiel, aber nur mit dem, was seine Fixture
        // hergibt — dort ist jede Buchung eine Umbuchung, also bleibt die Cashflow-Tabelle leer.
        // Dieser Test füttert den Store **direkt** mit einem Ergebnis, in dem **jede** der fünf
        // Detail-Listen etwas trägt, und liest alles zurück: so ist keine Tabelle davon abhängig,
        // wie die Beispieldaten gerade aussehen.
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = AuswertenSqliteStore::new(outbox.clone())?;
        // Der Kopf-`JOIN` in `stored_evaluations` verlangt ein reales Quell-Fenster
        // (Referenz-per-ID, ADR-0012); dessen Tabelle legt der `fenstern`-Store an.
        FensternSqliteStore::new(outbox.clone())?;
        outbox.with_connection(|conn| {
            conn.execute(
                "INSERT INTO auswertungsfenster (id, from_day, to_day, currency) \
                 VALUES ('w-1', 0, 0, 'EUR');",
                [],
            )
        })?;

        // Die Arbeitseinheit quittiert **eine** Nachricht; ohne sie failt der Commit als
        // `AlreadyProcessed` — Ablage und Ack hängen an derselben Transaktion (ADR-0012).
        let ack = outbox.enqueue(AUSWERTEN_CONSUMED, "w-1")?;

        let unit = store.begin(MessageId(&ack));
        unit.save(&WindowEvaluationRow {
            window_id: "w-1".to_string(),
            currency: "EUR".to_string(),
            months: vec![MonthlyCashflowRow {
                year: 2024,
                month: 6,
                incoming_minor_units: 250_000,
                outgoing_minor_units: 3_000,
            }],
            transfers: vec![InternalTransferRow {
                year: 2024,
                month: 6,
                role: "spar".to_string(),
                incoming_minor_units: 0,
                outgoing_minor_units: 50_000,
            }],
            fingerprints: vec![COMPANY_FINGERPRINT.to_string()],
            counter_accounts: vec![CounterAccountRow::Own {
                role: "spar".to_string(),
            }],
            categories: vec![auswerten_adapter::CategoryRow {
                fingerprint: COMPANY_FINGERPRINT.to_string(),
                category: "Abo".to_string(),
            }],
        })
        .expect("die Ablage muss gelingen");
        unit.commit().expect("der Commit muss gelingen");

        assert_eq!(
            stored_evaluations(&outbox)?,
            vec![StoredEvaluation {
                currency: "EUR".to_string(),
                months: vec![(2024, 6, 250_000, 3_000)],
                transfers: vec![(2024, 6, "spar".to_string(), 0, 50_000)],
                fingerprints: vec![COMPANY_FINGERPRINT.to_string()],
                counter_accounts: vec![("own".to_string(), Some("spar".to_string()))],
                categories: vec![(COMPANY_FINGERPRINT.to_string(), "Abo".to_string())],
            }],
            "jede der fünf Detail-Listen steht in ihrer eigenen Tabelle wieder"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn jede_fehlende_der_sechs_tabellen_failt_die_ablage_terminal() -> rusqlite::Result<()> {
        // Der Sechs-Tabellen-Beweis (Issue 0077, Kriterium 3): je Durchgang stehen **alle** Tabellen
        // bis auf **eine**. Weil der Preflight über **jedes** `TableSpec::insert` aus `TABLES` läuft,
        // failt schon `save` terminal — nicht erst der Commit als Retry. Fiele eine Tabelle aus
        // `TABLES` (oder würde ihr `insert` nicht mehr geprobt), wird dieser Test für genau sie rot,
        // während der Ausfall-Test über den ganz schemalosen Store weiter grün bliebe.
        TABLES.iter().try_for_each(|fehlend| {
            let outbox = SqliteOutbox::open_in_memory()?;
            // Bewusst an `new` vorbeigebaut, damit `new` die fehlende Tabelle nicht doch anlegt.
            outbox.with_connection(|conn| {
                TABLES
                    .iter()
                    .filter(|table| table.name != fehlend.name)
                    .try_for_each(|table| conn.execute_batch(table.ddl))
            })?;
            let store = AuswertenSqliteStore::without_schema(outbox.clone());
            let unit = store.begin(MessageId("egal"));

            let fehler = unit.save(&leeres_ergebnis()).expect_err(&format!(
                "eine fehlende {} muss save terminal failen lassen",
                fehlend.name
            ));
            assert!(
                !format!("{fehler}").is_empty(),
                "der terminale Store-Fehler trägt eine (redigierte) Diagnose"
            );
            Ok(())
        })
    }

    /// Ein leeres Fenster-Ergebnis: die Ablage-Wege sind hier nicht das Prüfobjekt, sondern der
    /// Preflight, der **vor** jedem Schreiben läuft.
    fn leeres_ergebnis() -> WindowEvaluationRow {
        WindowEvaluationRow {
            window_id: "w".to_string(),
            currency: "EUR".to_string(),
            months: vec![],
            transfers: vec![],
            fingerprints: vec![],
            counter_accounts: vec![],
            categories: vec![],
        }
    }
}
