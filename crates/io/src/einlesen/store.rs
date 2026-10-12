//! Der reale **einlesen-Ergebnis-Store** über `rusqlite`
//! ([Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md)) — der erste
//! slice-spezifische Store hinter dem öffentlichen [`StoreSeam`], auf dem transaktionalen Rückgrat aus
//! [Issue 0052](../../../../docs/issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md).
//! Er tritt an die Stelle des früheren In-Memory-Fakes; damit läuft der `einlesen`-Slice **end-to-end
//! ohne einen einzigen Fake** (echte CSV-Quelle → echte Geld-/Kalender-Dekodierung → echte Tabelle →
//! echter Broker), alles in **einer** Transaktion.
//!
//! **Atomarität (Tabelle = Wahrheit, Message = Zeiger, [ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)):**
//! [`EinlesenSqliteStore`] ist die [`UnitFactory`](crate::UnitFactory) des Slice — je
//! Nachricht öffnet es eine [`EinlesenUnit`]. Deren [`store`](StoreSeam::store) **mintet die
//! Zeilen-ID synchron an der DB** und **stagt** das Zeilen-`INSERT` in **dieselbe** committende
//! Transaktion wie der `Ingested`-Publish und der Ack — eine gestagete Schreibung committet mit der
//! Nachricht oder gar nicht. Die **DB vergibt die ID** (`randomblob`), und **genau sie** reist als
//! `partition_key` der `Ingested`-Nachricht weiter; das Aggregat referenziert nichts Fremdes außer
//! per diese ID.
//!
//! **Seit [Issue 0074](../../../../docs/issues/0074-storespec-abstraktion-einf-hren-und-einlesen-migrieren.md)
//! trägt dieser Store nur noch das Slice-Spezifische** ([ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md)):
//! sein [`StoreSpec`]-`impl` (Schema + Insert + Flatten + Row-mit-ID) und die dünne Seam-`impl`, die
//! [`SliceUnit::store_dto`](crate::sqlite::SliceUnit) aufruft. Store-Struct, `new`, Unit-Newtype,
//! Preflight/Minten, die [`UnitFactory`](crate::UnitFactory) und der Outbox-Passthrough leben
//! generisch in [`SliceStore`]/[`SliceUnit`] — die Unit-Quelle seit dem `auswerten`-Capstone
//! ([Issue 0077](../../../../docs/issues/0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md)),
//! der die vier strukturgleichen `UnitFactory`-Traits zu **einem** kollabiert hat.
//! Der Seam-Vertrag Richtung Slice ist davon **unberührt**
//! ([ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)).
//!
//! **Fehlbar + terminal (redigiert):** ein unerreichbarer Store (Schema fehlt) failt **synchron** in
//! `store` als [`StoreUnavailable`] (→ `SinkError::StoreFailed`) und damit terminal + redigiert (nur
//! io-Diagnose der Zugriffs-Mechanik, nie ein Zell-/Feldwert). Kein Dead-Letter, kein Retry im Slice.

use einlesen_adapter::{CalendarDay, Direction, Line, StoreOutcome, StoreSeam, StoreUnavailable};
use rusqlite::{Connection, Transaction, params};

use crate::sqlite::{MintedId, SliceStore, SliceUnit, StagedRow, StoreSpec, TableSpec};

/// Die `einlesen`-Persistenz als [`StoreSpec`]: **eine** Tabelle mit ihrem `INSERT`, und als fehlbare
/// Umdeutung nur die u64→i64-Deutung des Betrags. Der Marker-Typ parametrisiert
/// [`SliceStore`]/[`SliceUnit`] auf diesen Slice.
pub struct EinlesenLines;

impl StoreSpec for EinlesenLines {
    type Dto = Line;
    /// Die abgeflachte Zeile ohne ihre ID — schreibbar wird erst die `LineRow`, die `stage` daraus
    /// **mit** der geminteten ID baut.
    type Prepared = LineFields;
    type Error = StoreUnavailable;

    fn tables() -> &'static [TableSpec] {
        TABLES
    }

    fn flatten(line: &Line) -> Result<LineFields, StoreUnavailable> {
        LineFields::try_from(line)
    }

    fn stage(fields: LineFields, id: MintedId) -> StagedRow {
        let row = LineRow::new(id, fields);
        Box::new(move |tx| row.insert(tx))
    }

    /// Reicht die `rusqlite`-Diagnose eines misslungenen Store-Zugriffs durch (fehlendes Schema,
    /// DB-Fehler) — Mechanik, nie ein Zell-/Feldwert.
    fn unavailable(error: rusqlite::Error) -> StoreUnavailable {
        store_unavailable(&error.to_string())
    }
}

/// Der reale `einlesen`-Ergebnis-Store: der generische [`SliceStore`] über [`EinlesenLines`] und
/// damit die [`UnitFactory`](crate::UnitFactory) des Slice. `Clone` teilt dasselbe
/// Rückgrat (`Rc` im Innern).
pub type EinlesenSqliteStore = SliceStore<EinlesenLines>;

/// Eine `einlesen`-Arbeitseinheit: der gestagete Ergebnis-Speicher (`StoreSeam`) **und** die Outbox
/// (`OutboxPublisher`) in einem, atomar committbar (`UnitOfWork`).
pub type EinlesenUnit = SliceUnit<EinlesenLines>;

impl StoreSeam for EinlesenUnit {
    /// Fragt zuerst, ob der Bestand den `row_key` der Zeile schon trägt — dann legt er **nichts** an
    /// ([`StoreOutcome::AlreadyKnown`], ADR-0016). Sonst mintet er die Zeilen-ID **synchron an der
    /// DB** (`randomblob`) und stagt das Zeilen-`INSERT` in die committende Transaktion; ein
    /// unerreichbarer Store (Schema fehlt / DB-Fehler) failt **hier** terminal als redigierte
    /// [`StoreUnavailable`]. Der `INSERT` selbst committet mit Ack und `Ingested`-Publish oder gar
    /// nicht.
    ///
    /// Gefragt wird der **committete** Bestand: zwei gleiche Schlüssel **innerhalb** eines Batches
    /// sähe die Frage nicht — die weist schon die ACL des Slice ab, und die `UNIQUE`-Spalte bleibt
    /// das letzte Netz.
    fn store(&self, line: &Line) -> Result<StoreOutcome, StoreUnavailable> {
        if self.read(|conn| knows_row_key(conn, &line.row_key))? {
            return Ok(StoreOutcome::AlreadyKnown);
        }
        self.store_dto(line).map(StoreOutcome::Stored)
    }
}

/// Trägt der Bestand den Zeilen-Schlüssel schon?
fn knows_row_key(conn: &Connection, row_key: &str) -> rusqlite::Result<bool> {
    conn.query_row(SELECT_ROW_KEY_KNOWN, params![row_key], |row| row.get(0))
}

/// Das Präfix, das dieser Store **jeder** eigenen Diagnose voranstellt — die eine Quelle für den
/// Text, den [`store_unavailable`] setzt und den der Leak-Test **positiv** festnagelt
/// ([Issue 0079](../../../../docs/issues/0079-leak-tests-auf-das-diagnose-praefix-festnageln.md)):
/// fiele die durchgereichte Diagnose weg oder käme sie aus einer anderen Quelle, verschwände es aus
/// der Meldung und der Test würde rot.
const UNAVAILABLE_PREFIX: &str = "einlesen store unavailable";

/// Eine redigierte [`StoreUnavailable`] aus io-eigener Diagnose (Zugriffs-Mechanik, nie ein
/// Zell-/Feldwert — der Seam-Vertrag verlangt die Redaktion).
fn store_unavailable(message: &str) -> StoreUnavailable {
    StoreUnavailable {
        message: format!("{UNAVAILABLE_PREFIX}: {message}"),
    }
}

/// Die abgeflachte [`Line`] in ihrer besitzenden Speicher-Form — **ohne** ihre ID und darum **nicht**
/// schreibbar. Der einzige Ort, an dem dieser Store die Adapter-DTO anfasst; alles Fehlbare (die
/// u64→i64-Deutung des Betrags) ist danach entschieden.
///
/// `pub` aus Sichtbarkeits-Zwang (sie steht als `Prepared` in der Signatur eines öffentlichen
/// Trait-`impl`, `private_interfaces`), **nicht** als API: das Modul `store` ist privat, also bleibt
/// sie außerhalb von `io` nicht konstruierbar und nicht lesbar (benennbar ist sie als
/// `<EinlesenLines as StoreSpec>::Prepared` schon — nur eben inert).
pub struct LineFields {
    row_key: String,
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

impl TryFrom<&Line> for LineFields {
    type Error = StoreUnavailable;

    /// Fehlbar allein im Betrag: eine Magnitude jenseits `i64::MAX` passt nicht in die
    /// `INTEGER`-Spalte und failt terminal als redigierte [`StoreUnavailable`] (Speicher-Mechanik,
    /// nie ein Rohwert).
    fn try_from(line: &Line) -> Result<Self, StoreUnavailable> {
        Ok(Self {
            row_key: line.row_key.clone(),
            ordering_account: line.ordering_account.clone(),
            counter_account: line.counter_account.clone(),
            amount_minor_units: i64::try_from(line.amount_minor_units)
                .map_err(|_overflow| store_unavailable("amount out of range for storage"))?,
            currency: line.currency.clone(),
            direction: direction_token(line.direction),
            recipient_name: line.recipient_name.clone(),
            creditor_id: line.creditor_id.clone(),
            booking_day: line.booking_day,
            value_date: line.value_date,
            purpose: line.purpose.clone(),
            booking_text: line.booking_text.clone(),
        })
    }
}

/// Die SQL-fertige Zeile: die abgeflachten Felder **unter** ihrer DB-vergebenen ID. Sie reist in die
/// gestagete Ablage-Closure und wird erst am Commit-Rand geschrieben. Die ID ist erster Parameter von
/// [`new`](LineRow::new); dass es die eben an der DB gemintete ist, trägt ihr Typ
/// [`MintedId`] — eine von Hand gebaute Schein-ID ist hier ein Compile-Fehler.
struct LineRow {
    id: MintedId,
    fields: LineFields,
}

impl LineRow {
    const fn new(id: MintedId, fields: LineFields) -> Self {
        Self { id, fields }
    }

    fn insert(&self, tx: &Transaction<'_>) -> rusqlite::Result<()> {
        let fields = &self.fields;
        tx.execute(
            INSERT_LINE,
            params![
                self.id,
                fields.row_key,
                fields.ordering_account,
                fields.counter_account,
                fields.amount_minor_units,
                fields.currency,
                fields.direction,
                fields.recipient_name,
                fields.creditor_id,
                fields.booking_day.year,
                fields.booking_day.month,
                fields.booking_day.day,
                fields.value_date.year,
                fields.value_date.month,
                fields.value_date.day,
                fields.purpose,
                fields.booking_text,
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

/// Die **eine** Tabelle des Slice mit ihrem DDL und ihrem Ablage-`INSERT`. Anlegen (durch den
/// Schema-Besitzer, [`schema`](crate::schema)) und Preflight laufen beide über diese Liste — eine
/// Tabelle, die eines davon verpasst, ist gar nicht erst deklarierbar.
const TABLES: &[TableSpec] = &[TableSpec {
    name: "einlesen_lines",
    ddl: SCHEMA,
    insert: INSERT_LINE,
}];

/// Das Schema der `einlesen`-Ergebnis-Tabelle: die abgeflachte [`Line`] unter der DB-vergebenen ID.
/// Rohe Identifikatoren (IBANs, Klarname) liegen hier **roh** — das ist erlaubt (lokale Persistenz,
/// kein Egress; obfuskiert wird der nachgelagerte Slice, [ADR-0008](../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)).
///
/// **`row_key`** ([ADR-0016](../../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md))
/// ist der Zeilen-Schlüssel der Quelle — ein **Digest**, nie die Kundenreferenz oder die
/// zusammengesetzte Zeile im Klartext. Die Spalte ist `UNIQUE`: ein neuer Import ergänzt, eine schon
/// bekannte Zeile liegt nie zweimal hier. Der `UNIQUE`-Index trägt zugleich die Frage „kenne ich die
/// Zeile schon?" vor jeder Ablage.
///
/// `direction` ist enum-artig (nur `incoming`/`outgoing`) und wird per `CHECK`-Constraint auf
/// DB-Ebene erzwungen; `created_at`/`updated_at` (Epoch-Millis) tragen den Audit-Zeitstempel jeder
/// Zeile — beide Konventionen aus `.rules/sql/schema-konventionen.md`. Die Tabelle ist append-only,
/// daher fällt `updated_at` mit `created_at` zusammen.
const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS einlesen_lines (
    id               TEXT    PRIMARY KEY,
    row_key          TEXT    NOT NULL UNIQUE,
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

// Die beiden `unixepoch('subsec')`-Aufrufe liefern **denselben** Zeitpunkt: SQLite berechnet die
// aktuelle Zeit **einmal pro Statement**, sodass created_at und updated_at bei diesem append-only
// Insert garantiert zusammenfallen (empirisch bestätigt) — kein CTE nötig.
const INSERT_LINE: &str = "\
INSERT INTO einlesen_lines (
    id, row_key, ordering_account, counter_account, amount_minor_units, currency, direction,
    recipient_name, creditor_id,
    booking_year, booking_month, booking_day,
    value_year, value_month, value_day,
    purpose, booking_text,
    created_at, updated_at
) VALUES (
    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

/// Die Frage vor jeder Ablage: liegt dieser Zeilen-Schlüssel schon im Bestand? Getragen vom
/// `UNIQUE`-Index der Spalte.
const SELECT_ROW_KEY_KNOWN: &str =
    "SELECT EXISTS (SELECT 1 FROM einlesen_lines WHERE row_key = ?1);";

#[cfg(test)]
const SELECT_LINES: &str = "\
SELECT id, ordering_account, counter_account, amount_minor_units, currency, direction,
       recipient_name, creditor_id,
       booking_year, booking_month, booking_day,
       value_year, value_month, value_day,
       purpose, booking_text, row_key
FROM einlesen_lines
ORDER BY rowid;";

#[cfg(test)]
impl EinlesenSqliteStore {
    /// Liest die abgelegten Zeilen samt DB-vergebener ID zurück — **nur für Tests** (der reale
    /// Lesepfad des Folge-Slice liegt hinter dessen eigenem Seam, io-seitig, vertagt). Rekonstruiert
    /// die [`Line`] feld-treu, sodass ein Test die ganze DTO in **einem** `assert_eq!` vergleichen kann.
    fn lines(&self) -> rusqlite::Result<Vec<(String, Line)>> {
        self.with_connection(|conn| {
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
                        row_key: row.get(16)?,
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
    use crate::outbox::{OutboxError, OutboxMessage};
    use crate::pipeline_test_support::{assert_redacted_diagnosis, csv_source, register_fresh};
    use crate::sqlite::SqliteOutbox;
    use einlesen_adapter::SourceSeam;

    /// Die rohe Auftraggeber-IBAN der `gueltig.csv`-Fixture — sie darf in **keinem** Fehlergrund
    /// auftauchen (Datenschutz-Redaktion).
    const ROH_IBAN: &str = "DE89370400440532013000";

    /// Die erwartete `Line`-Projektion der einzigen Zeile aus `gueltig.csv` (EDEKA, `-50,00 EUR`) —
    /// unabhängige Wahrheit für den Feld-Treue-Vergleich der committeten Zeile. Den Zeilen-Schlüssel
    /// liefert die echte Quelle selbst (er ist ein Digest, kein lesbarer Wert): verglichen wird, dass
    /// er **unverändert** bis in die Tabelle reist.
    fn erwartete_gueltige_zeile() -> Line {
        let row_key = csv_source("gueltig.csv")
            .fetch()
            .ok()
            .and_then(|records| records.into_iter().next())
            .and_then(|record| record.row_key)
            .unwrap_or_default();
        Line {
            row_key,
            ordering_account: ROH_IBAN.to_string(),
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
    fn deliver(
        outbox: &SqliteOutbox,
        store: EinlesenSqliteStore,
        fixture: &str,
    ) -> rusqlite::Result<Vec<(OutboxMessage, Result<(), OutboxError>)>> {
        let source = csv_source(fixture);
        let konsument = consumer(
            &source,
            &GermanCalendar,
            &GermanMoney,
            &IsoCurrencies,
            store,
        );
        let mut dispatcher = Dispatcher::new();
        register_fresh(&mut dispatcher, CONSUMED_MESSAGE_TYPE, Box::new(konsument));
        outbox.enqueue(CONSUMED_MESSAGE_TYPE, fixture)?;
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
        let store = EinlesenSqliteStore::with_own_tables(outbox.clone())?;

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
        // Ein Store **ohne** seine Ergebnis-Tabelle (das Test-Rückgrat trägt nur die Outbox, und `new`
        // legt seit Issue 0084 kein Schema an) ist unerreichbar:
        // die Ablage failt synchron und terminal, bevor irgendetwas committet.
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = EinlesenSqliteStore::new(outbox.clone());

        let outcomes = deliver(&outbox, store, "gueltig.csv")?;

        // Terminal (Warning), redigiert — beidseitig festgenagelt (Issue 0079): der Grund trägt das
        // Präfix **dieses** Stores (positiv, also ist die durchgereichte Diagnose wirklich da) und
        // nie eine Roh-IBAN (negativ) …
        assert_redacted_diagnosis(&outcomes, UNAVAILABLE_PREFIX, ROH_IBAN);
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
        let store = EinlesenSqliteStore::with_own_tables(outbox.clone())?;

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
            let store = EinlesenSqliteStore::with_own_tables(outbox.clone())?;

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

    /// Der Zeilen-Schlüssel jeder abgelegten Zeile — getrennt von der ID, die je Ablage neu
    /// gemintet wird.
    fn row_keys(store: &EinlesenSqliteStore) -> rusqlite::Result<Vec<String>> {
        Ok(store
            .lines()?
            .into_iter()
            .map(|(_, line)| line.row_key)
            .collect())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ein_zweiter_import_derselben_datei_legt_nichts_an_und_publiziert_nichts()
    -> rusqlite::Result<()> {
        // Ein neuer Import ergänzt (ADR-0016): derselbe Auszug, zweimal zugestellt, hinterlässt
        // denselben Bestand — und keinen zweiten Zeiger, sonst legte `obfuskieren` die Zeile doppelt ab.
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = EinlesenSqliteStore::with_own_tables(outbox.clone())?;
        deliver(&outbox, store.clone(), "gueltig.csv")?;

        let outcomes = deliver(&outbox, store.clone(), "gueltig.csv")?;

        // (Der erste `Ingested`-Zeiger liegt in diesem Test ohne Abnehmer offen und wird erneut
        // zugestellt — gezählt wird hier nur der Auslöser.)
        assert!(
            outcomes.iter().any(|(message, outcome)| {
                message.message_type == CONSUMED_MESSAGE_TYPE && outcome.is_ok()
            }),
            "der zweite Auslöser wird ebenso atomar verarbeitet: {outcomes:?}"
        );
        assert_eq!(store.lines()?.len(), 1, "die Zeile liegt genau einmal da");
        assert_eq!(
            ingested_partition_keys(&outbox)?.len(),
            1,
            "und genau ein Zeiger reist stromabwärts"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn zwei_gleiche_buchungen_ohne_kundenreferenz_werden_beide_abgelegt() -> rusqlite::Result<()> {
        // Zweimal dasselbe Parkticket am selben Tag, Spalte für Spalte gleich: zwei echte Buchungen.
        // Die laufende Nummer im Zeilen-Schlüssel hält sie auseinander — beide zählen, auch nach
        // einem zweiten Import derselben Datei bleiben es zwei.
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = EinlesenSqliteStore::with_own_tables(outbox.clone())?;

        deliver(&outbox, store.clone(), "zwei_gleiche.csv")?;
        deliver(&outbox, store.clone(), "zwei_gleiche.csv")?;

        let keys = row_keys(&store)?;
        assert_eq!(
            keys.len(),
            2,
            "beide gleichen Buchungen liegen da: {keys:?}"
        );
        assert!(
            keys.first() != keys.get(1),
            "unter verschiedenen Schlüsseln"
        );
        assert_eq!(ingested_partition_keys(&outbox)?.len(), 2);
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_vorgemerkte_zeile_bricht_den_import_nicht_ab_und_wird_nicht_abgelegt()
    -> rusqlite::Result<()> {
        // Die vorgemerkte Zeile hat eine leere Valuta — ein Pflichtfeld. Sie wird übersprungen, bevor
        // geprüft wird; die gebuchte Zeile daneben kommt an.
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = EinlesenSqliteStore::with_own_tables(outbox.clone())?;

        let outcomes = deliver(&outbox, store.clone(), "vorgemerkt.csv")?;

        assert!(
            matches!(outcomes.as_slice(), [(_, Ok(()))]),
            "eine vorgemerkte Zeile bricht nichts ab: {outcomes:?}"
        );
        let lines = store.lines()?;
        assert!(
            matches!(lines.as_slice(), [(_, line)] if line.purpose.as_deref() == Some("Strom Juli")),
            "nur die gebuchte Zeile liegt da: {lines:?}"
        );
        assert_eq!(ingested_partition_keys(&outbox)?.len(), 1);
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn zwei_ueberlappende_auszuege_mit_wiederkehrender_referenz_verlieren_keinen_monat()
    -> rusqlite::Result<()> {
        // Der Repro aus Review-Runde 1, gegen echtes SQLite: April–Juni, dann Mai–Juli, dieselbe
        // Kundenreferenz und derselbe Betrag jeden Monat. Vier Monate, vier Zeilen — der Juli fehlt
        // nicht still.
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = EinlesenSqliteStore::with_own_tables(outbox.clone())?;

        deliver(&outbox, store.clone(), "referenz_wiederholt_a.csv")?;
        deliver(&outbox, store.clone(), "referenz_wiederholt_b.csv")?;

        let monate: Vec<u8> = store
            .lines()?
            .into_iter()
            .map(|(_, line)| line.booking_day.month)
            .collect();
        assert_eq!(monate, vec![6, 5, 4, 7]);
        Ok(())
    }
}
