//! Der reale **fenstern-Ergebnis-Store** über `rusqlite`
//! ([Issue 0057](../../../../docs/issues/0057-fenstern-store-real-und-end-to-end.md)) — der dritte
//! slice-spezifische Store hinter dem öffentlichen [`AnalysisWindowStoreSeam`], auf dem
//! transaktionalen Rückgrat aus
//! [Issue 0052](../../../../docs/issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md) und
//! **die Replikation des `obfuskierung`-Musters**
//! ([Issue 0056](../../../../docs/issues/0056-obfuskierung-store-real-und-end-to-end.md)). Er tritt an
//! die Stelle des `FakeDb`/`FakeUnit` der Fixtures; zusammen mit dem realen Lese-Seam
//! ([`ObfuskiertWindowSource`](crate::fenstern::ObfuskiertWindowSource)) läuft der `fenstern`-Slice
//! damit **end-to-end ohne einen einzigen Fake**.
//!
//! **Atomarität (Tabelle = Wahrheit, Message = Zeiger, [ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)):**
//! [`FensternSqliteStore`] ist die [`UnitFactory`](crate::fenstern::UnitFactory) des Slice — je
//! Nachricht öffnet es eine [`FensternUnit`] über einer [`SqliteUnit`]. Deren
//! [`save`](AnalysisWindowStoreSeam::save) **mintet die neue ID synchron an der DB** (`randomblob`)
//! und **stagt** das Fenster-`INSERT` **und** die Buchungs-Fremdschlüssel in **dieselbe** committende
//! Transaktion wie die ausgehenden Nachrichten (`WindowClosed`/`AdvanceWindow`/`HistoryComplete`)
//! und den Ack. **Genau die neue ID** reist als `partition_key` der `WindowClosed`-Nachricht weiter;
//! das Fenster referenziert seine Buchungen **nur per Fremdschlüssel** (nie eingebettet).
//!
//! **Zwei Tabellen:** `auswertungsfenster` trägt die Spanne (als `i64`-Tag-Stamps, dieselbe Achse wie
//! die Domäne) und die eine Währung; `auswertungsfenster_buchungen` die Fremdschlüssel auf die
//! obfuskierten Zeilen, **positionsgetreu** (die Reihenfolge, in der das Aggregat sie trägt).
//!
//! **Fehlbar + terminal:** ein unerreichbarer Store (Schema fehlt) failt **synchron** in `save` als
//! [`StoreUnavailable`] (→ Domänen-`WindowError::StorageUnavailable`), terminal. Kein Dead-Letter,
//! kein Retry im Slice.

use fenstern_adapter::{AnalysisWindowRow, AnalysisWindowStoreSeam, StoreUnavailable};
use rusqlite::{Transaction, params};

use crate::MessageId;
use crate::fenstern::UnitFactory;
use crate::outbox::{OutboxCommand, OutboxPublisher, UnitOfWork};
use crate::sqlite::{SqliteCommitError, SqliteOutbox, SqliteUnit};

/// Der reale `fenstern`-Ergebnis-Store: die [`UnitFactory`](crate::fenstern::UnitFactory) des Slice
/// über dem SQLite-Rückgrat. Hält den [`SqliteOutbox`] (Broker **und** Unit-Quelle) und stellt beim
/// Aufbau seine beiden Ergebnis-Tabellen sicher. `Clone` teilt dasselbe Rückgrat (`Rc` im Innern).
#[derive(Clone)]
pub struct FensternSqliteStore {
    outbox: SqliteOutbox,
}

impl FensternSqliteStore {
    /// Baut den Store über dem gegebenen Rückgrat und legt die `auswertungsfenster`-/
    /// `auswertungsfenster_buchungen`-Tabellen an (`IF NOT EXISTS` — ein bestehender Bestand bleibt
    /// erhalten). `ensure_columns` rüstet die Audit-Spalten auf einer **alten** Tabelle nach.
    pub fn new(outbox: SqliteOutbox) -> rusqlite::Result<Self> {
        outbox.with_connection(|conn| {
            conn.execute_batch(SCHEMA)?;
            crate::sqlite::ensure_columns(conn, "auswertungsfenster", crate::sqlite::AUDIT_COLUMNS)
        })?;
        Ok(Self { outbox })
    }
}

impl UnitFactory for FensternSqliteStore {
    type Unit = FensternUnit;

    fn begin(&self, ack: MessageId<'_>) -> FensternUnit {
        FensternUnit(self.outbox.begin(ack))
    }
}

/// Eine `fenstern`-Arbeitseinheit: der gestagete Ergebnis-Speicher (`AnalysisWindowStoreSeam`)
/// **und** die Outbox (`OutboxPublisher`) in einem, atomar committbar (`UnitOfWork`) — ein Newtype
/// über der generischen [`SqliteUnit`], der ihr das slice-spezifische Ablage-`INSERT` gibt und
/// Publish/Commit durchreicht.
pub struct FensternUnit(SqliteUnit);

impl AnalysisWindowStoreSeam for FensternUnit {
    /// Mintet die neue ID **synchron an der DB** (`randomblob`) und stagt das Fenster- und die
    /// Buchungs-`INSERT`s in die committende Transaktion; ein unerreichbarer Store (Schema fehlt /
    /// DB-Fehler) failt **hier** terminal als [`StoreUnavailable`].
    fn save(&self, window: &AnalysisWindowRow) -> Result<String, StoreUnavailable> {
        let row = WindowRow::try_from(window)?;
        let id: String = self
            .0
            .with_connection(|conn| {
                conn.prepare(INSERT_WINDOW).map(drop)?;
                conn.query_row(MINT_ID, [], |r| r.get(0))
            })
            .map_err(unavailable)?;
        let row = row.with_id(id.clone());
        self.0.stage(move |tx| row.insert(tx));
        Ok(id)
    }
}

impl OutboxPublisher for FensternUnit {
    fn publish<T: OutboxCommand>(&self, command: T) {
        self.0.publish(command);
    }
}

impl UnitOfWork for FensternUnit {
    type Error = SqliteCommitError;

    fn commit(self) -> Result<(), SqliteCommitError> {
        self.0.commit()
    }
}

fn unavailable(err: rusqlite::Error) -> StoreUnavailable {
    StoreUnavailable {
        message: format!("auswertungsfenster-Store nicht erreichbar: {err}"),
    }
}

/// Die abgeflachte [`AnalysisWindowRow`] in ihrer SQL-fertigen, **besitzenden** Form — sie reist in
/// die gestagete Ablage-Closure und wird erst am Commit-Rand geschrieben. Die `id` kommt erst nach
/// dem Minten dazu ([`with_id`](WindowRow::with_id)); `bookings` trägt die Fremdschlüssel bereits
/// **positionsgetreu** nummeriert.
struct WindowRow {
    id: String,
    from_day: i64,
    to_day: i64,
    currency: String,
    bookings: Vec<(i64, String)>,
}

impl TryFrom<&AnalysisWindowRow> for WindowRow {
    type Error = StoreUnavailable;

    /// Nummeriert die Buchungs-Fremdschlüssel positionsgetreu. `i64::try_from` auf die Position ist
    /// eine reine Speicher-Mechanik-Absicherung (ein Fenster mit mehr als `i64::MAX` Buchungen ist
    /// unrealistisch) und failt terminal als [`StoreUnavailable`], nie mit einem Rohwert.
    fn try_from(window: &AnalysisWindowRow) -> Result<Self, StoreUnavailable> {
        let bookings = window
            .booking_ids
            .iter()
            .enumerate()
            .map(|(position, booking_id)| {
                i64::try_from(position)
                    .map(|position| (position, booking_id.clone()))
                    .map_err(|_overflow| StoreUnavailable {
                        message: "Fenster trägt mehr Buchungen als sich ordnen lassen".to_string(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            id: String::new(),
            from_day: window.from,
            to_day: window.to,
            currency: window.currency.clone(),
            bookings,
        })
    }
}

impl WindowRow {
    /// Setzt die frisch gemintete DB-ID (die zugleich der publizierte Zeiger ist).
    fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    fn insert(&self, tx: &Transaction<'_>) -> rusqlite::Result<()> {
        tx.execute(
            INSERT_WINDOW,
            params![self.id, self.from_day, self.to_day, self.currency],
        )?;
        for (position, booking_id) in &self.bookings {
            tx.execute(INSERT_BOOKING, params![self.id, position, booking_id])?;
        }
        Ok(())
    }
}

/// Das Schema der beiden `fenstern`-Ergebnis-Tabellen: `auswertungsfenster` trägt die Spanne (als
/// `i64`-Tag-Stamps) und die eine Währung unter der DB-vergebenen ID;
/// `auswertungsfenster_buchungen` die Fremdschlüssel auf die obfuskierten Zeilen, **positionsgetreu**
/// (`window_id`, `position`) — **kein** DB-`FOREIGN KEY`-Constraint (SQLite erzwingt FKs nur mit
/// aktiviertem `PRAGMA foreign_keys`, das die synchrone Outbox-Spine bewusst nicht schaltet), aber
/// die **Referenz-Spalte `window_id` ist indiziert** (`.rules/sql/queries-und-performance.md`).
/// `created_at`/`updated_at` (Epoch-Millis) tragen den Audit-Zeitstempel; die Tabelle ist append-only,
/// darum fällt `updated_at` mit `created_at` zusammen.
const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS auswertungsfenster (
    id         TEXT    PRIMARY KEY,
    from_day   INTEGER NOT NULL,
    to_day     INTEGER NOT NULL,
    currency   TEXT    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
);
CREATE TABLE IF NOT EXISTS auswertungsfenster_buchungen (
    window_id  TEXT    NOT NULL,
    position   INTEGER NOT NULL,
    booking_id TEXT    NOT NULL,
    PRIMARY KEY (window_id, position)
);
CREATE INDEX IF NOT EXISTS auswertungsfenster_buchungen_window_id_idx
    ON auswertungsfenster_buchungen (window_id);";

const INSERT_WINDOW: &str = "\
INSERT INTO auswertungsfenster (id, from_day, to_day, currency, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

const INSERT_BOOKING: &str = "\
INSERT INTO auswertungsfenster_buchungen (window_id, position, booking_id) VALUES (?1, ?2, ?3);";

const MINT_ID: &str = "SELECT lower(hex(randomblob(16)));";

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::num::NonZeroU32;
    use std::rc::Rc;

    use application_kernel::PublishCommand;
    use chrono::{Datelike, NaiveDate};
    use fenstern_adapter::{HistoryCompleteMessage, WindowClosedMessage};
    use obfuskierung_adapter::ObfuscatedMessage;
    use rusqlite::params;

    use super::*;
    use crate::Dispatcher;
    use crate::einlesen::{
        CONSUMED_MESSAGE_TYPE as EINLESEN_CONSUMED, EinlesenSqliteStore, GermanCalendar,
        GermanMoney, IsoCurrencies, consumer as einlesen_consumer,
    };
    use crate::fenstern::{
        CONSUMED_MESSAGE_TYPE as FENSTERN_CONSUMED, ObfuskiertWindowSource, START_MESSAGE_TYPE,
        consumer as fenstern_consumer, start_consumer,
    };
    use crate::obfuskierung::{
        CONSUMED_MESSAGE_TYPE as OBFUSKIERUNG_CONSUMED, EinlesenRowSource, ObfuskierungSqliteStore,
        SeededObfuscator, consumer as obfuskierung_consumer,
    };
    use crate::outbox::{OutboxError, OutboxHandler, OutboxMessage};

    const SEED: u64 = 42;

    /// Die `message_type`s der stromabwärtigen Nachrichten — aus den kanonischen
    /// `PublishCommand::MESSAGE_TYPE`s der DTOs statt als String-Literal, damit eine Umbenennung
    /// nicht still an den Tests vorbeidriftet.
    const OBFUSCATED_MESSAGE_TYPE: &str = <ObfuscatedMessage as PublishCommand>::MESSAGE_TYPE;
    const WINDOW_CLOSED_MESSAGE_TYPE: &str = <WindowClosedMessage as PublishCommand>::MESSAGE_TYPE;
    const HISTORY_COMPLETE_MESSAGE_TYPE: &str =
        <HistoryCompleteMessage as PublishCommand>::MESSAGE_TYPE;

    /// Baut den echten CSV-CAMT-v8-Reader auf die benannte `einlesen`-Fixture (Pfad relativ zum
    /// io-Crate) — dieselben Fixtures wie der `obfuskierung`-Store-Test.
    fn csv_source(name: &str) -> crate::einlesen::CsvCamtV8Source {
        let path = format!(
            "{}/fixtures/einlesen/csv/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        crate::einlesen::CsvCamtV8Source::new(path)
    }

    /// Der `i64`-Tag-Stamp eines Kalendertags, wie ihn dieser Store ablegt — dasselbe
    /// `num_days_from_ce`, das der reale Lese-Seam ([`super::super::source`]) benutzt.
    fn stamp(year: i32, month: u32, day: u32) -> i64 {
        i64::from(
            NaiveDate::from_ymd_opt(year, month, day)
                .expect("gültiges Testdatum")
                .num_days_from_ce(),
        )
    }

    /// Ein Zähl-/Schluck-Consumer: ackt jede Nachricht ohne eigene Wirkung, merkt sich aber
    /// `(message_type, partition_key)` — für Nachrichtentypen, die dieser Test bewusst nicht
    /// stromabwärts konsumiert (`Obfuscated`/`WindowClosed`/`HistoryComplete`), deren Auftreten er
    /// aber belegen will. Ohne ihn blieben diese Nachrichten für immer offen und die Broker-Schleife
    /// würde nie konvergieren.
    struct Capture<'a> {
        outbox: &'a SqliteOutbox,
        seen: Rc<RefCell<Vec<(String, String)>>>,
    }

    impl OutboxHandler for Capture<'_> {
        fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
            self.seen
                .borrow_mut()
                .push((message.message_type.clone(), message.partition_key.clone()));
            self.outbox
                .begin(MessageId(&message.id))
                .commit()
                .map_err(|err| OutboxError::Retry {
                    reason: err.to_string(),
                })
        }
    }

    /// Treibt die Outbox, bis keine offenen Nachrichten mehr übrig sind — nötig für `fenstern`s
    /// Selbst-Fortsetzung (jede Seite reiht die nächste ein). Bricht wie ein realer Broker beim
    /// ersten Fehler in einer Runde ab (keine halb-fortgesetzte Kette), statt ihn zu wiederholen.
    /// Gedeckelt: ein nicht terminierender Pager muss den Test laut scheitern lassen, statt ihn
    /// hängen zu lassen.
    #[expect(
        clippy::panic_in_result_fn,
        clippy::panic,
        reason = "Test-Helfer: ein nicht terminierender Pager ist ein Regress und muss hart scheitern"
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

    /// Fährt die reale zweistufige Vorstufe auf **einem** Rückgrat: `einlesen` über der Fixture,
    /// dann `obfuskierung` über **allen** dabei publizierten `Ingested`-Nachrichten.
    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! prüfen die Vorstufen-Invarianten vor dem fenstern-Lauf"
    )]
    fn seed_obfuskiert_bestand(outbox: &SqliteOutbox, fixture: &str) -> rusqlite::Result<()> {
        let einlesen_store = EinlesenSqliteStore::new(outbox.clone())?;
        let einlesen_source = csv_source(fixture);
        let einlesen = einlesen_consumer(
            &einlesen_source,
            &GermanCalendar,
            &GermanMoney,
            &IsoCurrencies,
            einlesen_store,
        );
        let mut stage1 = Dispatcher::new();
        assert!(
            stage1
                .register(EINLESEN_CONSUMED, Box::new(einlesen))
                .is_none(),
            "frische Registrierung darf keinen Consumer verdrängen"
        );
        outbox.enqueue(EINLESEN_CONSUMED, fixture)?;
        let einlesen_out = outbox.deliver_open(&stage1)?;
        assert!(
            einlesen_out.iter().all(|(_, outcome)| outcome.is_ok()),
            "einlesen muss atomar verarbeiten: {einlesen_out:?}"
        );

        let source = EinlesenRowSource::new(outbox.clone());
        let obfuscator = SeededObfuscator;
        let store = ObfuskierungSqliteStore::new(outbox.clone())?;
        let obfuskierung = obfuskierung_consumer(&source, &obfuscator, SEED, store);
        let mut stage2 = Dispatcher::new();
        assert!(
            stage2
                .register(OBFUSKIERUNG_CONSUMED, Box::new(obfuskierung))
                .is_none(),
            "frische Registrierung darf keinen Consumer verdrängen"
        );
        let obfuskierung_out = outbox.deliver_open(&stage2)?;
        assert!(
            obfuskierung_out.iter().all(|(_, outcome)| outcome.is_ok()),
            "obfuskierung muss atomar verarbeiten: {obfuskierung_out:?}"
        );
        Ok(())
    }

    /// Ein zurückgelesenes Fenster (nur für Tests): `(from_day, to_day, currency, booking_ids)`.
    type StoredWindow = (i64, i64, String, Vec<String>);

    /// Liest die abgelegten Fenster zurück (nur für Tests): je Fenster Spanne, Währung und die
    /// positionsgetreuen Buchungs-Fremdschlüssel.
    fn stored_windows(outbox: &SqliteOutbox) -> rusqlite::Result<Vec<StoredWindow>> {
        outbox.with_connection(|conn| {
            let mut statement = conn.prepare(
                "SELECT id, from_day, to_day, currency FROM auswertungsfenster ORDER BY rowid;",
            )?;
            let windows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            windows
                .into_iter()
                .map(|(id, from_day, to_day, currency)| {
                    let mut buchungen = conn.prepare(
                        "SELECT booking_id FROM auswertungsfenster_buchungen \
                         WHERE window_id = ?1 ORDER BY position;",
                    )?;
                    let booking_ids = buchungen
                        .query_map(params![id], |row| row.get(0))?
                        .collect::<rusqlite::Result<Vec<String>>>()?;
                    Ok((from_day, to_day, currency, booking_ids))
                })
                .collect()
        })
    }

    /// Die reale, DB-gemintete obfuskiert-ID zu einem Betrag — die Fixture trägt vier eindeutige
    /// Beträge, über die sich die opaken IDs den vier CSV-Zeilen zuordnen lassen.
    fn obfuskiert_id_by_amount(
        outbox: &SqliteOutbox,
        amount_minor_units: i64,
    ) -> rusqlite::Result<String> {
        outbox.with_connection(|conn| {
            conn.query_row(
                "SELECT id FROM obfuskiert WHERE amount_minor_units = ?1;",
                params![amount_minor_units],
                |row| row.get(0),
            )
        })
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn mehrere_seiten_werden_real_paginiert_gespeichert_und_schliessen_mit_historie_vollstaendig()
    -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        seed_obfuskiert_bestand(&outbox, "mehrseitig.csv")?;

        // Die vier realen, DB-geminteten obfuskiert-IDs — über den je CSV-Zeile eindeutigen Betrag
        // zugeordnet (A=20.06., B=19.06., C=15.06., D=10.06.).
        let id_a = obfuskiert_id_by_amount(&outbox, 1000)?;
        let id_b = obfuskiert_id_by_amount(&outbox, 2000)?;
        let id_c = obfuskiert_id_by_amount(&outbox, 3000)?;
        let id_d = obfuskiert_id_by_amount(&outbox, 4000)?;

        let source = ObfuskiertWindowSource::new(outbox.clone());
        let store = FensternSqliteStore::new(outbox.clone())?;
        let window_days = NonZeroU32::new(3).expect("Seitengröße ist nicht null");

        let seen = Rc::new(RefCell::new(Vec::new()));
        let mut dispatcher = Dispatcher::new();
        assert!(
            dispatcher
                .register(
                    OBFUSCATED_MESSAGE_TYPE,
                    Box::new(Capture {
                        outbox: &outbox,
                        seen: Rc::clone(&seen)
                    }),
                )
                .is_none()
                && dispatcher
                    .register(
                        WINDOW_CLOSED_MESSAGE_TYPE,
                        Box::new(Capture {
                            outbox: &outbox,
                            seen: Rc::clone(&seen)
                        }),
                    )
                    .is_none()
                && dispatcher
                    .register(
                        HISTORY_COMPLETE_MESSAGE_TYPE,
                        Box::new(Capture {
                            outbox: &outbox,
                            seen: Rc::clone(&seen)
                        }),
                    )
                    .is_none()
                && dispatcher
                    .register(
                        START_MESSAGE_TYPE,
                        Box::new(start_consumer(&source, store.clone())),
                    )
                    .is_none()
                && dispatcher
                    .register(
                        FENSTERN_CONSUMED,
                        Box::new(fenstern_consumer(window_days, &source, store)),
                    )
                    .is_none(),
            "frische Registrierungen dürfen keinen Consumer verdrängen"
        );

        outbox.enqueue(START_MESSAGE_TYPE, "")?;
        let outcomes = drain(&outbox, &dispatcher)?;

        assert!(
            outcomes.iter().all(|(_, outcome)| outcome.is_ok()),
            "der ganze Lauf muss ohne Fehler durchlaufen: {outcomes:?}"
        );

        // Drei Seiten, neu→alt, jede nicht-leer; die Lücken zwischen den Buchungstagen werden
        // übersprungen, weil jeder Folge-Anker ein echter Buchungstag ist (Selbst-Fortsetzung).
        assert_eq!(
            stored_windows(&outbox)?,
            vec![
                (
                    stamp(2024, 6, 18),
                    stamp(2024, 6, 20),
                    "EUR".to_string(),
                    vec![id_a.clone(), id_b.clone()]
                ),
                (
                    stamp(2024, 6, 13),
                    stamp(2024, 6, 15),
                    "EUR".to_string(),
                    vec![id_c.clone()]
                ),
                (
                    stamp(2024, 6, 8),
                    stamp(2024, 6, 10),
                    "EUR".to_string(),
                    vec![id_d.clone()]
                ),
            ],
            "drei Seiten, neu→alt, mit den erwarteten Fremdschlüsseln je Seite"
        );

        let window_closed = seen
            .borrow()
            .iter()
            .filter(|(typ, _)| typ == WINDOW_CLOSED_MESSAGE_TYPE)
            .count();
        assert_eq!(
            window_closed, 3,
            "je materialisiertem Fenster ein WindowClosed-Zeiger"
        );

        let history_complete: Vec<String> = seen
            .borrow()
            .iter()
            .filter(|(typ, _)| typ == HISTORY_COMPLETE_MESSAGE_TYPE)
            .map(|(_, key)| key.clone())
            .collect();
        assert_eq!(
            history_complete,
            vec![format!("{}:{}", stamp(2024, 6, 8), stamp(2024, 6, 20))],
            "genau ein HistoryComplete mit der Lauf-Spanne (Boden der letzten Seite bis Lauf-Gipfel)"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ein_leerer_bestand_endet_erfolgreich_ohne_fenster() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        // Die obfuskiert-Tabelle existiert (für den Lese-Seam), ist aber leer.
        ObfuskierungSqliteStore::new(outbox.clone())?;

        let source = ObfuskiertWindowSource::new(outbox.clone());
        let store = FensternSqliteStore::new(outbox.clone())?;
        let window_days = NonZeroU32::new(3).expect("Seitengröße ist nicht null");

        let mut dispatcher = Dispatcher::new();
        assert!(
            dispatcher
                .register(
                    START_MESSAGE_TYPE,
                    Box::new(start_consumer(&source, store.clone())),
                )
                .is_none()
                && dispatcher
                    .register(
                        FENSTERN_CONSUMED,
                        Box::new(fenstern_consumer(window_days, &source, store)),
                    )
                    .is_none(),
            "frische Registrierungen dürfen keinen Consumer verdrängen"
        );

        outbox.enqueue(START_MESSAGE_TYPE, "")?;
        let outcomes = drain(&outbox, &dispatcher)?;

        assert!(
            matches!(outcomes.as_slice(), [(_, Ok(()))]),
            "nur der Anstoß läuft, er publiziert nichts weiter: {outcomes:?}"
        );
        assert!(
            stored_windows(&outbox)?.is_empty(),
            "ein leerer Bestand legt kein Fenster ab"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_fehlende_obfuskiert_tabelle_failt_die_anker_frage_terminal() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        // Bewusst **keine** obfuskiert-Tabelle angelegt (kein ObfuskierungSqliteStore::new).
        let source = ObfuskiertWindowSource::new(outbox.clone());
        let store = FensternSqliteStore::new(outbox.clone())?;

        let mut dispatcher = Dispatcher::new();
        assert!(
            dispatcher
                .register(START_MESSAGE_TYPE, Box::new(start_consumer(&source, store)))
                .is_none(),
            "frische Registrierung darf keinen Consumer verdrängen"
        );

        outbox.enqueue(START_MESSAGE_TYPE, "")?;
        let outcomes = outbox.deliver_open(&dispatcher)?;

        assert!(
            matches!(outcomes.as_slice(), [(_, Err(OutboxError::Warning { .. }))]),
            "eine fehlende obfuskiert-Tabelle muss die Anker-Frage terminal + redigiert failen \
             lassen: {outcomes:?}"
        );
        assert!(stored_windows(&outbox)?.is_empty());
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ein_unerreichbarer_fenster_store_failt_terminal_und_redigiert_ohne_zu_publizieren()
    -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        seed_obfuskiert_bestand(&outbox, "gueltig.csv")?;

        let source = ObfuskiertWindowSource::new(outbox.clone());
        // Bewusst an `new` vorbeigebaut: die Ergebnis-Tabellen fehlen → unerreichbar.
        let store = FensternSqliteStore {
            outbox: outbox.clone(),
        };
        let window_days = NonZeroU32::new(3).expect("Seitengröße ist nicht null");

        let seen = Rc::new(RefCell::new(Vec::new()));
        let mut dispatcher = Dispatcher::new();
        assert!(
            dispatcher
                .register(
                    OBFUSCATED_MESSAGE_TYPE,
                    Box::new(Capture {
                        outbox: &outbox,
                        seen: Rc::clone(&seen)
                    }),
                )
                .is_none()
                && dispatcher
                    .register(
                        START_MESSAGE_TYPE,
                        Box::new(start_consumer(&source, store.clone())),
                    )
                    .is_none()
                && dispatcher
                    .register(
                        FENSTERN_CONSUMED,
                        Box::new(fenstern_consumer(window_days, &source, store)),
                    )
                    .is_none(),
            "frische Registrierungen dürfen keinen Consumer verdrängen"
        );

        outbox.enqueue(START_MESSAGE_TYPE, "")?;
        let outcomes = drain(&outbox, &dispatcher)?;

        // Der Grund trägt die redigierte io-Diagnose, nie eine rohe Buchungs-ID.
        assert!(
            matches!(outcomes.last(), Some((_, Err(OutboxError::Warning { .. })))),
            "ein unerreichbarer Fenster-Store muss den Fenster-Schritt terminal failen lassen: \
             {outcomes:?}"
        );
        Ok(())
    }
}
