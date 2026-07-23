//! Der reale **einlesen-Ergebnis-Store** über `rusqlite`
//! ([Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md)) — der erste
//! slice-spezifische Store hinter dem öffentlichen [`StoreSeam`], auf dem transaktionalen Rückgrat aus
//! [Issue 0052](../../../../docs/issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md).
//! Er tritt an die Stelle des früheren In-Memory-Fakes; damit läuft der `einlesen`-Slice **end-to-end
//! ohne einen einzigen Fake** (echte CSV-Quelle → echte Geld-/Kalender-Dekodierung → echte Tabelle →
//! echter Broker), alles in **einer** Transaktion.
//!
//! **Atomarität (Tabelle = Wahrheit, Message = Zeiger, [ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)):**
//! [`EinlesenSqliteStore`] ist die [`UnitFactory`](crate::einlesen::UnitFactory) des Slice — je
//! Nachricht öffnet es eine [`EinlesenUnit`] über einer [`SqliteUnit`]. Deren
//! [`store`](StoreSeam::store) **mintet die Zeilen-ID synchron an der DB** und **stagt** das
//! Zeilen-`INSERT` in **dieselbe** committende Transaktion wie der `Ingested`-Publish und der Ack —
//! eine gestagete Schreibung committet mit der Nachricht oder gar nicht. Die **DB vergibt die ID**
//! (`randomblob`), und **genau sie** reist als `partition_key` der `Ingested`-Nachricht weiter; das
//! Aggregat referenziert nichts Fremdes außer per diese ID.
//!
//! **Fehlbar + terminal (redigiert):** ein unerreichbarer Store (Schema fehlt) failt **synchron** in
//! `store` als [`StoreUnavailable`] (→ `SinkError::StoreFailed`) und damit terminal + redigiert (nur
//! io-Diagnose der Zugriffs-Mechanik, nie ein Zell-/Feldwert). Kein Dead-Letter, kein Retry im Slice.

use einlesen_adapter::{CalendarDay, Direction, Line, StoreSeam, StoreUnavailable};
use rusqlite::{Transaction, params};

use crate::MessageId;
use crate::einlesen::UnitFactory;
use crate::outbox::{OutboxCommand, OutboxPublisher, UnitOfWork};
use crate::sqlite::{SqliteCommitError, SqliteOutbox, SqliteUnit};

/// Der reale `einlesen`-Ergebnis-Store: die [`UnitFactory`](crate::einlesen::UnitFactory) des Slice
/// über dem SQLite-Rückgrat. Hält den [`SqliteOutbox`] (Broker **und** Unit-Quelle) und stellt beim
/// Aufbau seine Ergebnis-Tabelle sicher. `Clone` teilt dasselbe Rückgrat (`Rc` im Innern).
#[derive(Clone)]
pub struct EinlesenSqliteStore {
    outbox: SqliteOutbox,
}

impl EinlesenSqliteStore {
    /// Baut den Store über dem gegebenen Rückgrat und legt die `einlesen`-Ergebnis-Tabelle an
    /// (`IF NOT EXISTS` — ein bestehender Bestand bleibt erhalten).
    pub fn new(outbox: SqliteOutbox) -> rusqlite::Result<Self> {
        outbox.with_connection(|conn| conn.execute_batch(SCHEMA))?;
        Ok(Self { outbox })
    }
}

impl UnitFactory for EinlesenSqliteStore {
    type Unit = EinlesenUnit;

    fn begin(&self, ack: MessageId<'_>) -> EinlesenUnit {
        EinlesenUnit(self.outbox.begin(ack))
    }
}

/// Eine `einlesen`-Arbeitseinheit: der gestagete Ergebnis-Speicher (`StoreSeam`) **und** die Outbox
/// (`OutboxPublisher`) in einem, atomar committbar (`UnitOfWork`) — ein Newtype über der generischen
/// [`SqliteUnit`], der ihr das slice-spezifische Ablage-`INSERT` gibt und Publish/Commit durchreicht.
pub struct EinlesenUnit(SqliteUnit);

impl StoreSeam for EinlesenUnit {
    /// Mintet die Zeilen-ID **synchron an der DB** (`randomblob`) und stagt das Zeilen-`INSERT` in die
    /// committende Transaktion; ein unerreichbarer Store (Schema fehlt / DB-Fehler) failt **hier**
    /// terminal als redigierte [`StoreUnavailable`]. Der `INSERT` selbst committet mit Ack und
    /// `Ingested`-Publish oder gar nicht.
    fn store(&self, line: &Line) -> Result<String, StoreUnavailable> {
        let amount_minor_units = i64::try_from(line.amount_minor_units)
            .map_err(|_overflow| store_unavailable("amount out of range for storage"))?;
        let id: String = self
            .0
            .with_connection(|conn| {
                // Preflight: ist der Store erreichbar (Schema vorhanden)? Ein fehlendes Schema failt
                // **hier** terminal, nicht erst als Commit-Retry — die ID wird an der DB gemintet.
                conn.prepare(INSERT_LINE).map(drop)?;
                conn.query_row(MINT_ID, [], |row| row.get(0))
            })
            .map_err(store_unavailable_io)?;
        let row = LineRow::new(id.clone(), line, amount_minor_units);
        self.0.stage(move |tx| row.insert(tx));
        Ok(id)
    }
}

/// Eine redigierte [`StoreUnavailable`] aus io-eigener Diagnose (Zugriffs-Mechanik, nie ein
/// Zell-/Feldwert — der Seam-Vertrag verlangt die Redaktion).
fn store_unavailable(message: &str) -> StoreUnavailable {
    StoreUnavailable {
        message: format!("einlesen store unavailable: {message}"),
    }
}

/// Reicht die `rusqlite`-Diagnose eines misslungenen Store-Zugriffs durch (fehlendes Schema,
/// DB-Fehler) — Mechanik, nie ein Zell-/Feldwert.
fn store_unavailable_io(error: rusqlite::Error) -> StoreUnavailable {
    StoreUnavailable {
        message: format!("einlesen store unavailable: {error}"),
    }
}

/// Die abgeflachte [`Line`] in ihrer SQL-fertigen, **besitzenden** Form — sie reist in die gestagete
/// Ablage-Closure und wird erst am Commit-Rand geschrieben.
struct LineRow {
    id: String,
    ordering_account: String,
    counter_account: Option<String>,
    amount_minor_units: i64,
    currency: String,
    direction: &'static str,
    recipient_name: Option<String>,
    creditor_id: Option<String>,
    booking_day: CalendarDay,
    value_date: CalendarDay,
    purpose: Option<String>,
    booking_text: Option<String>,
}

impl LineRow {
    fn new(id: String, line: &Line, amount_minor_units: i64) -> Self {
        Self {
            id,
            ordering_account: line.ordering_account.clone(),
            counter_account: line.counter_account.clone(),
            amount_minor_units,
            currency: line.currency.clone(),
            direction: direction_token(line.direction),
            recipient_name: line.recipient_name.clone(),
            creditor_id: line.creditor_id.clone(),
            booking_day: line.booking_day,
            value_date: line.value_date,
            purpose: line.purpose.clone(),
            booking_text: line.booking_text.clone(),
        }
    }

    fn insert(&self, tx: &Transaction<'_>) -> rusqlite::Result<()> {
        tx.execute(
            INSERT_LINE,
            params![
                self.id,
                self.ordering_account,
                self.counter_account,
                self.amount_minor_units,
                self.currency,
                self.direction,
                self.recipient_name,
                self.creditor_id,
                self.booking_day.year,
                self.booking_day.month,
                self.booking_day.day,
                self.value_date.year,
                self.value_date.month,
                self.value_date.day,
                self.purpose,
                self.booking_text,
            ],
        )
        .map(drop)
    }
}

/// Der stabile Speicher-Token einer [`Direction`].
fn direction_token(direction: Direction) -> &'static str {
    match direction {
        Direction::Incoming => "incoming",
        Direction::Outgoing => "outgoing",
    }
}

impl OutboxPublisher for EinlesenUnit {
    fn publish<T: OutboxCommand>(&self, command: T) {
        self.0.publish(command);
    }
}

impl UnitOfWork for EinlesenUnit {
    type Error = SqliteCommitError;

    fn commit(self) -> Result<(), SqliteCommitError> {
        self.0.commit()
    }
}

/// Das Schema der `einlesen`-Ergebnis-Tabelle: die abgeflachte [`Line`] unter der DB-vergebenen ID.
/// Rohe Identifikatoren (IBANs, Klarname) liegen hier **roh** — das ist erlaubt (lokale Persistenz,
/// kein Egress; obfuskiert wird der nachgelagerte Slice, [ADR-0008](../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)).
///
/// `direction` ist enum-artig (nur `incoming`/`outgoing`) und wird per `CHECK`-Constraint auf
/// DB-Ebene erzwungen; `created_at`/`updated_at` (Epoch-Millis) tragen den Audit-Zeitstempel jeder
/// Zeile — beide Konventionen aus `.rules/sql/schema-konventionen.md`. Die Tabelle ist append-only,
/// daher fällt `updated_at` mit `created_at` zusammen.
const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS einlesen_lines (
    id               TEXT    PRIMARY KEY,
    ordering_account TEXT    NOT NULL,
    counter_account  TEXT,
    amount_minor_units      INTEGER NOT NULL,
    currency         TEXT    NOT NULL,
    direction        TEXT    NOT NULL CHECK (direction IN ('incoming', 'outgoing')),
    recipient_name   TEXT,
    creditor_id      TEXT,
    booking_year     INTEGER NOT NULL,
    booking_month    INTEGER NOT NULL,
    booking_day      INTEGER NOT NULL,
    value_year       INTEGER NOT NULL,
    value_month      INTEGER NOT NULL,
    value_day        INTEGER NOT NULL,
    purpose          TEXT,
    booking_text     TEXT,
    created_at       INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at       INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
);";

const INSERT_LINE: &str = "\
INSERT INTO einlesen_lines (
    id, ordering_account, counter_account, amount_minor_units, currency, direction,
    recipient_name, creditor_id,
    booking_year, booking_month, booking_day,
    value_year, value_month, value_day,
    purpose, booking_text
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16);";

const MINT_ID: &str = "SELECT lower(hex(randomblob(16)));";

#[cfg(test)]
const SELECT_LINES: &str = "\
SELECT id, ordering_account, counter_account, amount_minor_units, currency, direction,
       recipient_name, creditor_id,
       booking_year, booking_month, booking_day,
       value_year, value_month, value_day,
       purpose, booking_text
FROM einlesen_lines
ORDER BY rowid;";

#[cfg(test)]
impl EinlesenSqliteStore {
    /// Liest die abgelegten Zeilen samt DB-vergebener ID zurück — **nur für Tests** (der reale
    /// Lesepfad des Folge-Slice liegt hinter dessen eigenem Seam, io-seitig, vertagt). Rekonstruiert
    /// die [`Line`] feld-treu, sodass ein Test die ganze DTO in **einem** `assert_eq!` vergleichen kann.
    fn lines(&self) -> rusqlite::Result<Vec<(String, Line)>> {
        self.outbox.with_connection(|conn| {
            let mut statement = conn.prepare(SELECT_LINES)?;
            let rows = statement.query_map([], |row| {
                // Die Währung ist der kanonische ISO-Code als Text — feld-treu zurückgelesen, ohne
                // Enum-Deutung (die Domäne führt keine Währungstabelle mehr).
                let currency = row.get::<_, String>(4)?;
                let direction = match row.get::<_, String>(5)?.as_str() {
                    "incoming" => Direction::Incoming,
                    "outgoing" => Direction::Outgoing,
                    other => return Err(unexpected_token(5, other)),
                };
                let amount_minor_units = u64::try_from(row.get::<_, i64>(3)?)
                    .map_err(|_negative| unexpected_token(3, "negative amount"))?;
                Ok((
                    row.get::<_, String>(0)?,
                    Line {
                        ordering_account: row.get(1)?,
                        counter_account: row.get(2)?,
                        amount_minor_units,
                        currency,
                        direction,
                        recipient_name: row.get(6)?,
                        creditor_id: row.get(7)?,
                        booking_day: CalendarDay {
                            year: row.get(8)?,
                            month: row.get(9)?,
                            day: row.get(10)?,
                        },
                        value_date: CalendarDay {
                            year: row.get(11)?,
                            month: row.get(12)?,
                            day: row.get(13)?,
                        },
                        purpose: row.get(14)?,
                        booking_text: row.get(15)?,
                    },
                ))
            })?;
            rows.collect()
        })
    }
}

/// Ein beim Zurücklesen unerwarteter Enum-Token (kann nur bei manuell verfälschtem Bestand feuern) —
/// als `rusqlite`-Konvertierungsfehler; der Token ist ein interner Marker (`"EUR"`/`"incoming"`), nie
/// ein Roh-Zell­wert.
#[cfg(test)]
fn unexpected_token(column: usize, token: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        rusqlite::types::Type::Text,
        format!("unexpected stored enum token: {token}").into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dispatcher;
    use crate::einlesen::{
        CONSUMED_MESSAGE_TYPE, GermanCalendar, GermanMoney, IsoCurrencies, consumer,
    };
    use crate::outbox::OutboxError;
    use crate::sqlite::SqliteOutbox;

    /// Baut den echten CSV-CAMT-v8-Reader auf die benannte Fixture-Datei (Pfad relativ zum io-Crate).
    fn csv_source(name: &str) -> crate::einlesen::CsvCamtV8Source {
        let path = format!(
            "{}/fixtures/einlesen/csv/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        crate::einlesen::CsvCamtV8Source::new(path)
    }

    /// Die erwartete `Line`-Projektion der einzigen Zeile aus `gueltig.csv` (EDEKA, `-50,00 EUR`) —
    /// unabhängige Wahrheit für den Feld-Treue-Vergleich der committeten Zeile.
    fn erwartete_gueltige_zeile() -> Line {
        Line {
            ordering_account: "DE89370400440532013000".to_string(),
            counter_account: Some("DE11520513735120710131".to_string()),
            amount_minor_units: 5000,
            currency: "EUR".to_string(),
            direction: Direction::Outgoing,
            recipient_name: Some("EDEKA SAGT DANKE".to_string()),
            creditor_id: Some("DE98ZZZ09999999999".to_string()),
            booking_day: CalendarDay {
                year: 2024,
                month: 6,
                day: 13,
            },
            value_date: CalendarDay {
                year: 2024,
                month: 6,
                day: 14,
            },
            purpose: Some("Einkauf".to_string()),
            booking_text: Some("LASTSCHRIFT".to_string()),
        }
    }

    /// Registriert den `einlesen`-Consumer über die gegebene Fixture-Datei und dem realen Store und
    /// stellt **einen** eingereihten `ReadTransactions`-Auslöser über den echten Broker zu.
    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: das assert! prüft die Registrierungs-Invariante vor dem Dispatch"
    )]
    fn deliver(
        outbox: &SqliteOutbox,
        store: EinlesenSqliteStore,
        fixture: &str,
    ) -> rusqlite::Result<Vec<(String, Result<(), OutboxError>)>> {
        let source = csv_source(fixture);
        let konsument = consumer(
            &source,
            &GermanCalendar,
            &GermanMoney,
            &IsoCurrencies,
            store,
        );
        let mut dispatcher = Dispatcher::new();
        assert!(
            dispatcher
                .register(CONSUMED_MESSAGE_TYPE, Box::new(konsument))
                .is_none(),
            "frische Registrierung darf keinen Consumer verdrängen"
        );
        outbox.enqueue(CONSUMED_MESSAGE_TYPE, "auszug-juni")?;
        outbox.deliver_open(&dispatcher)
    }

    /// Zählt die `Ingested`-Nachrichten und gibt ihre `partition_key`s (die Zeiger) zurück.
    fn ingested_partition_keys(outbox: &SqliteOutbox) -> rusqlite::Result<Vec<String>> {
        outbox.with_connection(|conn| {
            let mut statement = conn.prepare(
                "SELECT partition_key FROM outbox WHERE message_type = 'Ingested' ORDER BY rowid;",
            )?;
            let rows = statement.query_map([], |row| row.get(0))?;
            rows.collect()
        })
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert_eq! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_fixture_zeile_landet_feldtreu_in_der_tabelle_und_publiziert_einen_zeiger()
    -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = EinlesenSqliteStore::new(outbox.clone())?;

        let outcomes = deliver(&outbox, store.clone(), "gueltig.csv")?;

        // Der Auslöser ist atomar verarbeitet (geackt) — genau ein Ok-Ausgang.
        assert!(
            matches!(outcomes.as_slice(), [(_, Ok(()))]),
            "der Auslöser muss atomar verarbeitet werden: {outcomes:?}"
        );
        // Die eine Fixture-Zeile liegt feld-treu in der echten Tabelle (ganze DTO in einem Vergleich).
        let lines = store.lines()?;
        assert!(
            matches!(lines.as_slice(), [(_, line)] if *line == erwartete_gueltige_zeile()),
            "die committete Zeile muss feld-treu der erwarteten Projektion entsprechen: {lines:?}"
        );
        // Genau eine `Ingested`-Nachricht, deren Zeiger sich mit der DB-vergebenen Zeilen-ID deckt
        // (Tabelle = Wahrheit, Message = Pointer) — atomar in derselben Transaktion committet.
        let line_ids: Vec<String> = lines.iter().map(|(id, _)| id.clone()).collect();
        assert_eq!(
            ingested_partition_keys(&outbox)?,
            line_ids,
            "genau eine Ingested-Nachricht, deren partition_key die Zeilen-ID ist"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ein_unerreichbarer_store_failt_terminal_und_redigiert_ohne_zu_publizieren()
    -> rusqlite::Result<()> {
        // Ein Store **ohne** seine Ergebnis-Tabelle (bewusst an `new` vorbeigebaut) ist unerreichbar:
        // die Ablage failt synchron und terminal, bevor irgendetwas committet.
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = EinlesenSqliteStore {
            outbox: outbox.clone(),
        };
        let leck = "DE89370400440532013000";

        let outcomes = deliver(&outbox, store, "gueltig.csv")?;

        // Terminal (Warning), redigiert: der Grund benennt die Store-Mechanik, nie eine Roh-IBAN …
        assert!(
            matches!(
                outcomes.as_slice(),
                [(_, Err(OutboxError::Warning { reason }))]
                    if reason.contains("store") && !reason.contains(leck)
            ),
            "unerreichbarer Store muss terminal + redigiert failen: {outcomes:?}"
        );
        // … und **nichts** ist publiziert (kein Dead-Letter/Retry, keine halbe Kette).
        assert!(
            ingested_partition_keys(&outbox)?.is_empty(),
            "ein terminaler Store-Fehler darf nichts publizieren"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_kaputte_zeile_im_batch_laesst_nichts_halbes_zurueck() -> rusqlite::Result<()> {
        // Der Batch trägt eine gültige **und** eine kaputte Zeile (Pflichtfeld leer). Der Handler baut
        // erst **alle** Aggregate — die kaputte Zeile steigt aus, bevor die erste Ablage passiert.
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = EinlesenSqliteStore::new(outbox.clone())?;

        let outcomes = deliver(&outbox, store.clone(), "batch_eine_kaputt.csv")?;

        assert!(
            matches!(outcomes.as_slice(), [(_, Err(OutboxError::Warning { .. }))]),
            "eine kaputte Zeile failt den Batch terminal: {outcomes:?}"
        );
        // Batch-atomar: **weder** die gültige Zeile ist abgelegt **noch** eine Nachricht publiziert.
        assert!(
            store.lines()?.is_empty(),
            "keine Zeile darf den terminalen Batch-Fehler überleben"
        );
        assert!(
            ingested_partition_keys(&outbox)?.is_empty(),
            "keine Nachricht darf den terminalen Batch-Fehler überleben"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn nicht_zwei_stellen_waehrungen_skalieren_real_per_iso_4217() -> rusqlite::Result<()> {
        // Der **reale** Währungs-Seam (`IsoCurrencies` über `rusty_money`, ISO 4217) skaliert
        // währungs-korrekt end-to-end durch die echte Kette (CSV → Geld-/Währungs-Seam → Bridge →
        // Store): JPY hat **0** Nachkommastellen (`1234` → 1234 Minor Units), BHD **3** (`1,234` → 1234)
        // — **nicht** die feste Skala 2. Am committeten Betrag abgelesen, nicht als io-Unit-Test.
        for (fixture, currency) in [("jpy.csv", "JPY"), ("bhd.csv", "BHD")] {
            let outbox = SqliteOutbox::open_in_memory()?;
            let store = EinlesenSqliteStore::new(outbox.clone())?;

            let outcomes = deliver(&outbox, store.clone(), fixture)?;

            assert!(
                matches!(outcomes.as_slice(), [(_, Ok(()))]),
                "der {fixture}-Auslöser muss atomar verarbeitet werden: {outcomes:?}"
            );
            // Beide Rohbeträge tragen dieselben vier Ziffern; korrekt skaliert stehen sie als 1234
            // Minor Units da — unter Skala 2 wären JPY um Faktor 100, BHD um Faktor 10 verschoben.
            let lines = store.lines()?;
            assert!(
                matches!(lines.as_slice(), [(_, line)]
                    if line.amount_minor_units == 1234 && line.currency == currency),
                "der Betrag muss auf die ISO-Minor-Unit skaliert committen (1234), Währung {currency:?}: {lines:?}"
            );
        }
        Ok(())
    }
}
