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
//! [`AuswertenSqliteStore`] ist die [`UnitFactory`](crate::auswerten::UnitFactory) des Slice — je
//! Nachricht öffnet es eine [`AuswertenUnit`] über einer [`SqliteUnit`]. Deren
//! [`save`](EvaluationStoreSeam::save) **mintet die neue ID synchron an der DB** (`randomblob`) und
//! **stagt** die Ergebnis-`INSERT`s (Kopf + vier Detail-Tabellen) in **dieselbe** committende
//! Transaktion wie der Ack. Diese Reichweite **publiziert keinen Zeiger** stromabwärts (sie ist das
//! Pipeline-Ende bis zum `report`), darum vergibt der Seam die neue ID nur DB-intern und gibt sie
//! **nicht** zurück; das Ergebnis referenziert sein Quell-Fenster **nur per `window_id`**
//! (Referenz-per-ID auf `auswertungsfenster.id`), nie eingebettet.
//!
//! **Fünf Tabellen (Kopf + vier Detail-Listen):** `window_evaluations` trägt Fremdschlüssel, Währung
//! und Audit; die vier Detail-Tabellen die **positionsgetreuen** Listen — Monats-Buckets,
//! Fingerabdrücke, eingeordnete Gegenkonten und die Orakel-Kategorien. Jede Detail-Zeile referenziert
//! den Kopf **per `evaluation_id`** (indiziert), **kein** DB-`FOREIGN KEY`-Constraint (SQLite erzwingt
//! FKs nur mit `PRAGMA foreign_keys`, das die synchrone Outbox-Spine bewusst nicht schaltet).
//!
//! **Fehlbar + terminal:** ein unerreichbarer Store (Schema fehlt) failt **synchron** in `save` als
//! [`StoreUnavailable`] (→ Domänen-`EvaluationError::StorageUnavailable`), terminal. Kein Dead-Letter,
//! kein Retry im Slice.

use auswerten_adapter::{
    CounterAccountRow, EvaluationStoreSeam, MonthlyCashflowRow, StoreUnavailable,
    WindowEvaluationRow,
};
use rusqlite::{Transaction, params};

use crate::MessageId;
use crate::auswerten::UnitFactory;
use crate::outbox::{OutboxCommand, OutboxPublisher, UnitOfWork};
use crate::sqlite::{SqliteCommitError, SqliteOutbox, SqliteUnit, sqlite_backed_unit};

/// Der reale `auswerten`-Ergebnis-Store: die [`UnitFactory`](crate::auswerten::UnitFactory) des Slice
/// über dem SQLite-Rückgrat. Hält den [`SqliteOutbox`] (Broker **und** Unit-Quelle) und stellt beim
/// Aufbau seine fünf Ergebnis-Tabellen sicher. `Clone` teilt dasselbe Rückgrat (`Rc` im Innern).
#[derive(Clone)]
pub struct AuswertenSqliteStore {
    outbox: SqliteOutbox,
}

impl AuswertenSqliteStore {
    /// Baut den Store über dem gegebenen Rückgrat und legt die fünf `window_evaluation*`-Tabellen an
    /// (`IF NOT EXISTS` — ein bestehender Bestand bleibt erhalten). `ensure_columns` rüstet die
    /// Audit-Spalten auf **jeder** Tabelle nach, falls sie aus einer **alten** DB-Datei ohne sie
    /// stammen.
    pub fn new(outbox: SqliteOutbox) -> rusqlite::Result<Self> {
        outbox.with_connection(|conn| {
            conn.execute_batch(SCHEMA)?;
            for table in TABLES {
                crate::sqlite::ensure_columns(conn, table, crate::sqlite::AUDIT_COLUMNS)?;
            }
            Ok(())
        })?;
        Ok(Self { outbox })
    }
}

impl UnitFactory for AuswertenSqliteStore {
    type Unit = AuswertenUnit;

    fn begin(&self, ack: MessageId<'_>) -> AuswertenUnit {
        AuswertenUnit(self.outbox.begin(ack))
    }
}

/// Eine `auswerten`-Arbeitseinheit: der gestagete Ergebnis-Speicher (`EvaluationStoreSeam`) **und**
/// die Outbox (`OutboxPublisher`) in einem, atomar committbar (`UnitOfWork`) — ein Newtype über der
/// generischen [`SqliteUnit`], der ihr das slice-spezifische Ablage-`INSERT` gibt und Publish/Commit
/// durchreicht. Dass der Publish-Rand dieser Reichweite ungenutzt bleibt, ändert die Einheit nicht:
/// sie umspannt Ack und Ablage und wächst ohne Umbau um den späteren `report`-Anstoß.
pub struct AuswertenUnit(SqliteUnit);

impl EvaluationStoreSeam for AuswertenUnit {
    /// Mintet die neue ID **synchron an der DB** (`randomblob`) und stagt Kopf- und Detail-`INSERT`s
    /// in die committende Transaktion; ein unerreichbarer Store (Schema fehlt / DB-Fehler) failt
    /// **hier** terminal als [`StoreUnavailable`]. Der Preflight probt **alle fünf** Ablage-Statements —
    /// fehlt auch nur eine Detail-Tabelle, failt es hier terminal statt erst als Commit-Retry. Die
    /// u64→i64-Deutungen (Monats-Beträge, Positionen) laufen **vor** dem Minten, damit die schreibbare
    /// Zeile in einem Zug mit der ID entsteht.
    fn save(&self, evaluation: &WindowEvaluationRow) -> Result<(), StoreUnavailable> {
        let staged = StagedEvaluation::try_from(evaluation)?;
        let id: String = self
            .0
            .with_connection(|conn| {
                for statement in DETAIL_INSERTS {
                    conn.prepare(statement).map(drop)?;
                }
                crate::sqlite::mint_id(conn, INSERT_EVALUATION)
            })
            .map_err(unavailable)?;
        let staged = staged.with_id(id);
        self.0.stage(move |tx| staged.insert(tx));
        Ok(())
    }
}

sqlite_backed_unit!(AuswertenUnit);

fn unavailable(err: rusqlite::Error) -> StoreUnavailable {
    StoreUnavailable {
        message: format!("window_evaluations-Store nicht erreichbar: {err}"),
    }
}

/// Deutet einen **vorzeichenlosen** Betrag (Cashflow-Bucket) in die `i64`-Spalte. Eine Magnitude
/// jenseits `i64::MAX` ist unrealistisch und failt terminal als [`StoreUnavailable`]
/// (Speicher-Mechanik, nie ein Rohwert) — dieselbe Linie, die `obfuskierung` beim Betrag zieht.
fn magnitude(value: u64) -> Result<i64, StoreUnavailable> {
    i64::try_from(value).map_err(|_overflow| StoreUnavailable {
        message: "Cashflow-Betrag jenseits des darstellbaren Bereichs".to_string(),
    })
}

/// Die abgeflachte [`WindowEvaluationRow`] in ihrer SQL-fertigen, **besitzenden** Form — sie reist in
/// die gestagete Ablage-Closure und wird erst am Commit-Rand geschrieben. Ihre eigene DB-`id` kommt
/// erst mit [`with_id`](StagedEvaluation::with_id) hinzu (das Minten läuft nach dem fehlbaren
/// Umdeuten); eine Zeile mit leerer Platzhalter-ID ist unkonstruierbar.
struct StagedEvaluation {
    id: String,
    window_id: String,
    currency: String,
    months: Vec<MonthRow>,
    fingerprints: Vec<String>,
    counter_accounts: Vec<CounterRow>,
    categories: Vec<CategoryRow>,
}

/// Ein Monats-Bucket in seiner Speicher-Form: die Cashflow-Beträge schon als `i64` vorgeprüft.
struct MonthRow {
    year: i64,
    month: i64,
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

impl TryFrom<&WindowEvaluationRow> for StagedEvaluation {
    type Error = StoreUnavailable;

    /// Deutet die Adapter-Zeile in die besitzende Speicher-Form — **hier** liegen die fehlbaren
    /// u64→i64-Umdeutungen der Monats-Beträge (Positionen vergibt erst [`insert`](StagedEvaluation::insert)
    /// per `enumerate`). Die ID ist noch leer; [`with_id`](StagedEvaluation::with_id) setzt sie nach
    /// dem Minten.
    fn try_from(evaluation: &WindowEvaluationRow) -> Result<Self, StoreUnavailable> {
        let months = evaluation
            .months
            .iter()
            .map(MonthRow::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            id: String::new(),
            window_id: evaluation.window_id.clone(),
            currency: evaluation.currency.clone(),
            months,
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

impl StagedEvaluation {
    /// Setzt die frisch geminteten DB-`id` (das Ergebnis vergibt keinen publizierten Zeiger, die ID
    /// bleibt darum DB-intern und bindet nur die Detail-Zeilen an ihren Kopf).
    fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    /// Schreibt Kopf- und Detail-Zeilen in die committende Transaktion, jede Detail-Liste
    /// **positionsgetreu** (`enumerate`).
    fn insert(&self, tx: &Transaction<'_>) -> rusqlite::Result<()> {
        tx.execute(
            INSERT_EVALUATION,
            params![self.id, self.window_id, self.currency],
        )?;
        for (index, month) in self.months.iter().enumerate() {
            tx.execute(
                INSERT_MONTH,
                params![
                    self.id,
                    position_or_sql_error(index)?,
                    month.year,
                    month.month,
                    month.incoming_minor_units,
                    month.outgoing_minor_units,
                ],
            )?;
        }
        for (index, fingerprint) in self.fingerprints.iter().enumerate() {
            tx.execute(
                INSERT_FINGERPRINT,
                params![self.id, position_or_sql_error(index)?, fingerprint],
            )?;
        }
        for (index, counter_account) in self.counter_accounts.iter().enumerate() {
            tx.execute(
                INSERT_COUNTER_ACCOUNT,
                params![
                    self.id,
                    position_or_sql_error(index)?,
                    counter_account.kind,
                    counter_account.role,
                ],
            )?;
        }
        for (index, category) in self.categories.iter().enumerate() {
            tx.execute(
                INSERT_CATEGORY,
                params![
                    self.id,
                    position_or_sql_error(index)?,
                    category.fingerprint,
                    category.category,
                ],
            )?;
        }
        Ok(())
    }
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

/// Die fünf Tabellen, deren Audit-Spalten [`AuswertenSqliteStore::new`] nachrüstet.
const TABLES: [&str; 5] = [
    "window_evaluations",
    "window_evaluation_months",
    "window_evaluation_fingerprints",
    "window_evaluation_counter_accounts",
    "window_evaluation_categories",
];

/// Die vier Detail-`INSERT`s, die der Preflight in `save` allesamt probt (eine fehlende Detail-Tabelle
/// soll terminal in `save` failen, nicht erst als Commit-Retry).
const DETAIL_INSERTS: [&str; 4] = [
    INSERT_MONTH,
    INSERT_FINGERPRINT,
    INSERT_COUNTER_ACCOUNT,
    INSERT_CATEGORY,
];

/// Das Schema der fünf `auswerten`-Ergebnis-Tabellen. Der Kopf `window_evaluations` trägt den
/// **Fremdschlüssel** `window_id` auf `auswertungsfenster` (indiziert) und die Währung; die vier
/// Detail-Tabellen die positionsgetreuen Listen, jede per `evaluation_id` (indiziert) an ihren Kopf
/// gebunden. `kind` der Gegenkonten ist per `CHECK` auf `own`/`foreign` erzwungen; `created_at`/
/// `updated_at` (Epoch-Millis) tragen den Audit-Zeitstempel — die Tabellen sind append-only, `updated_at`
/// fällt mit `created_at` zusammen (`.rules/sql/schema-konventionen.md`).
const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS window_evaluations (
    id         TEXT    PRIMARY KEY,
    window_id  TEXT    NOT NULL,
    currency   TEXT    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
);
CREATE INDEX IF NOT EXISTS window_evaluations_window_id_idx
    ON window_evaluations (window_id);
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
    ON window_evaluation_months (evaluation_id);
CREATE TABLE IF NOT EXISTS window_evaluation_fingerprints (
    evaluation_id TEXT    NOT NULL,
    position      INTEGER NOT NULL,
    fingerprint   TEXT    NOT NULL,
    created_at    INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at    INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    PRIMARY KEY (evaluation_id, position)
);
CREATE INDEX IF NOT EXISTS window_evaluation_fingerprints_evaluation_id_idx
    ON window_evaluation_fingerprints (evaluation_id);
CREATE TABLE IF NOT EXISTS window_evaluation_counter_accounts (
    evaluation_id TEXT    NOT NULL,
    position      INTEGER NOT NULL,
    kind          TEXT    NOT NULL CHECK (kind IN ('own', 'foreign')),
    role          TEXT,
    created_at    INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at    INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    PRIMARY KEY (evaluation_id, position)
);
CREATE INDEX IF NOT EXISTS window_evaluation_counter_accounts_evaluation_id_idx
    ON window_evaluation_counter_accounts (evaluation_id);
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
    use crate::Dispatcher;
    use crate::auswerten::source::seed_marker;
    use crate::auswerten::{
        AuswertungsfensterSource, CONSUMED_HISTORY_MESSAGE_TYPE,
        CONSUMED_MESSAGE_TYPE as AUSWERTEN_CONSUMED, OccurrenceSink,
        consumer as auswerten_consumer, history_consumer,
    };
    use crate::common::calendar;
    use crate::fenstern::{
        CONSUMED_MESSAGE_TYPE as FENSTERN_CONSUMED, FensternSqliteStore, ObfuskiertWindowSource,
        START_MESSAGE_TYPE, consumer as fenstern_consumer, start_consumer,
    };
    use crate::outbox::{OutboxError, OutboxHandler, OutboxMessage};
    use crate::pipeline_test_support::{SEED, Stage, register_fresh, seed_through};

    const OBFUSCATED_MESSAGE_TYPE: &str = <ObfuscatedMessage as PublishCommand>::MESSAGE_TYPE;

    /// Der Fingerabdruck aller vier Buchungen der `mehrseitig.csv`: die Gegenpartei ist eine Firma
    /// mit fester Gläubiger-ID (kein Durchleiter), also teilen sie **einen** Fingerabdruck.
    const COMPANY_FINGERPRINT: &str = "COMPANY:DE98ZZZ09999999999";

    /// Der `i64`-Tag-Stamp eines Kalendertags über den geteilten [`calendar`]-Helfer (Tage seit der
    /// Unix-Epoche) — dieselbe Achse, die die Domäne für die Vorkommen liefert.
    fn stamp(year: i32, month: u32, day: u32) -> i64 {
        calendar::day_stamp(&format!("{year:04}-{month:02}-{day:02}")).expect("gültiges Testdatum")
    }

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

    /// Ackt eine Nachricht ohne eigene Wirkung — für Typen, die dieser Test bewusst nicht konsumiert
    /// (die von `obfuskierung` offen gelassenen `Obfuscated`-Zeiger). Ohne ihn blieben sie offen und
    /// die Broker-Schleife würde als „kein Consumer" (Critical) abbrechen.
    struct AckOnly<'a> {
        outbox: &'a SqliteOutbox,
    }
    impl OutboxHandler for AckOnly<'_> {
        fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
            self.outbox
                .begin(MessageId(&message.id))
                .commit()
                .map_err(|err| OutboxError::Retry {
                    reason: err.to_string(),
                })
        }
    }

    /// Treibt die Outbox, bis keine offenen Nachrichten mehr übrig sind (nötig für `fenstern`s
    /// Selbst-Fortsetzung). Bricht wie ein realer Broker beim ersten Fehler ab; gedeckelt gegen einen
    /// nicht terminierenden Lauf.
    #[expect(
        clippy::panic_in_result_fn,
        clippy::panic,
        reason = "Test-Helfer: ein nicht terminierender Lauf ist ein Regress und muss hart scheitern"
    )]
    fn drain(
        outbox: &SqliteOutbox,
        dispatcher: &Dispatcher<'_>,
    ) -> rusqlite::Result<Vec<(String, Result<(), OutboxError>)>> {
        let mut all = Vec::new();
        for _ in 0..32 {
            let batch = outbox.deliver_open(dispatcher)?;
            if batch.is_empty() {
                return Ok(all);
            }
            let failed = batch.iter().any(|(_, outcome)| outcome.is_err());
            all.extend(batch);
            if failed {
                return Ok(all);
            }
        }
        panic!("Broker-Schleife terminierte nicht (32 Runden)");
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
            Box::new(AckOnly { outbox: &outbox }),
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

        // Pro-Fenster-Reichweite: drei Fenster, jedes mit seinem Monats-Cashflow, dem einen
        // Fingerabdruck, dem als eigen (giro) aufgelösten Gegenkonto und ohne Kategorien.
        let own_giro = ("own".to_string(), Some("giro".to_string()));
        assert_eq!(
            stored_evaluations(&outbox)?,
            vec![
                StoredEvaluation {
                    currency: "EUR".to_string(),
                    months: vec![(2024, 6, 0, 3000)],
                    fingerprints: vec![COMPANY_FINGERPRINT.to_string()],
                    counter_accounts: vec![own_giro.clone()],
                    categories: vec![],
                },
                StoredEvaluation {
                    currency: "EUR".to_string(),
                    months: vec![(2024, 6, 0, 3000)],
                    fingerprints: vec![COMPANY_FINGERPRINT.to_string()],
                    counter_accounts: vec![own_giro.clone()],
                    categories: vec![],
                },
                StoredEvaluation {
                    currency: "EUR".to_string(),
                    months: vec![(2024, 6, 0, 4000)],
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
        let store = AuswertenSqliteStore {
            outbox: outbox.clone(),
        };
        let unit = store.begin(MessageId("egal"));

        let evaluation = WindowEvaluationRow {
            window_id: "w".to_string(),
            currency: "EUR".to_string(),
            months: vec![],
            fingerprints: vec![],
            counter_accounts: vec![],
            categories: vec![],
        };

        let fehler = unit
            .save(&evaluation)
            .expect_err("ein fehlendes Schema muss save terminal failen lassen");
        assert!(
            !format!("{fehler}").is_empty(),
            "der terminale Store-Fehler trägt eine (redigierte) Diagnose"
        );
        Ok(())
    }
}
