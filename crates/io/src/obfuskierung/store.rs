//! Der reale **obfuskierung-Ergebnis-Store** über `rusqlite`
//! ([Issue 0056](../../../../docs/issues/0056-obfuskierung-store-real-und-end-to-end.md)) — der
//! zweite slice-spezifische Store hinter dem öffentlichen [`ObfuscatedStoreSeam`], auf dem
//! transaktionalen Rückgrat aus
//! [Issue 0052](../../../../docs/issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md) und
//! **die Replikation des `einlesen`-Musters** ([Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md))
//! über die Datenschutz-Grenze. Er tritt an die Stelle des `pub(crate)`-Fakes der Specs; damit läuft
//! der `obfuskierung`-Slice **end-to-end ohne einen einzigen Fake** (reale `einlesen`-Zeile → realer
//! seed-basierter Obfuskator → reale `obfuskiert`-Tabelle → echter Broker), alles in **einer**
//! Transaktion.
//!
//! **Atomarität (Tabelle = Wahrheit, Message = Zeiger, [ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)):**
//! [`ObfuskierungSqliteStore`] ist die [`UnitFactory`](crate::UnitFactory) des Slice —
//! je Nachricht öffnet es eine [`ObfuskierungUnit`] über einer [`SqliteUnit`]. Deren
//! [`save`](ObfuscatedStoreSeam::save) **mintet die neue ID synchron an der DB** (`randomblob`) und
//! **stagt** das Zeilen-`INSERT` in **dieselbe** committende Transaktion wie der `Obfuscated`-Publish
//! und der Ack — eine gestagete Schreibung committet mit der Nachricht oder gar nicht. **Genau die
//! neue ID** reist als `partition_key` der `Obfuscated`-Nachricht weiter; die Zeile referenziert die
//! Quell-Buchung **nur per `source_id`** (Referenz-per-ID auf `einlesen_lines.id`), nie als
//! eingebettetes Roh-Aggregat.
//!
//! **Datenschutz-Grenze:** in diese Tabelle fließen **nur Token** — die Konten stehen als
//! `KONTO_…`/`PERSON_…`, Freitext als obfuskierter Text; **keine rohe IBAN**. Einziger Klartext-Anker
//! ist die öffentliche `Gläubiger-ID` einer Firma (ADR-0010). Der Seed reist **mit** jeder Zeile (für
//! die Rehydrierung), bleibt aber lokal — kein Egress-DTO trägt ihn.
//!
//! **Seit [Issue 0075](../../../../docs/issues/0075-obfuskierung-migrieren-und-storeunavailable-vereinheitlichen.md)
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
//! `save` als [`StoreUnavailable`] (→ Domänen-`ObfuscationError::StorageUnavailable` mit durchgereichtem
//! `reason`), terminal und redigiert — die Diagnose trägt seit Issue 0075 **denselben** redigierten
//! `message` wie die drei anderen Slices: nur io-Zugriffs-Mechanik, nie ein
//! Zell-/Feldwert. Dass sie hier keinen Zellwert tragen **kann**, ist strukturell: [`unavailable`](StoreSpec::unavailable)
//! ist allein aus dem Preflight/Mint-Zweig von [`store_dto`](crate::sqlite::SliceUnit) erreichbar, der
//! keine DTO-Daten anfasst. Kein Dead-Letter, kein Retry im Slice.

use obfuskierung_adapter::{ObfuscatedBooking, ObfuscatedStoreSeam, Recipient, StoreUnavailable};
use rusqlite::{Transaction, params};

use crate::sqlite::{MintedId, SliceStore, SliceUnit, StagedRow, StoreSpec, TableSpec};

/// Die `obfuskierung`-Persistenz als [`StoreSpec`]: **eine** Tabelle mit ihrem `INSERT`, und als
/// fehlbare Umdeutung nur die u64→i64-Deutung der Betrags-Magnitude. Der Marker-Typ parametrisiert
/// [`SliceStore`]/[`SliceUnit`] auf diesen Slice.
pub struct ObfuskiertBookings;

impl StoreSpec for ObfuskiertBookings {
    type Dto = ObfuscatedBooking;
    /// Die abgeflachte Buchung ohne ihre ID — schreibbar wird erst die `BookingRow`, die `stage`
    /// daraus **mit** der geminteten ID baut.
    type Prepared = BookingFields;
    type Error = StoreUnavailable;

    fn tables() -> &'static [TableSpec] {
        TABLES
    }

    fn flatten(booking: &ObfuscatedBooking) -> Result<BookingFields, StoreUnavailable> {
        BookingFields::try_from(booking)
    }

    fn stage(fields: BookingFields, id: MintedId) -> StagedRow {
        let row = BookingRow::new(id, fields);
        Box::new(move |tx| row.insert(tx))
    }

    /// Reicht die `rusqlite`-Diagnose eines misslungenen Store-Zugriffs durch (fehlendes Schema,
    /// DB-Fehler) — Mechanik, nie ein Zell-/Feldwert.
    fn unavailable(error: rusqlite::Error) -> StoreUnavailable {
        store_unavailable(&error.to_string())
    }
}

/// Der reale `obfuskierung`-Ergebnis-Store: der generische [`SliceStore`] über [`ObfuskiertBookings`]
/// und damit die [`UnitFactory`](crate::UnitFactory) des Slice. `Clone` teilt dasselbe
/// Rückgrat (`Rc` im Innern).
pub type ObfuskierungSqliteStore = SliceStore<ObfuskiertBookings>;

/// Eine `obfuskierung`-Arbeitseinheit: der gestagete Ergebnis-Speicher (`ObfuscatedStoreSeam`)
/// **und** die Outbox (`OutboxPublisher`) in einem, atomar committbar (`UnitOfWork`).
pub type ObfuskierungUnit = SliceUnit<ObfuskiertBookings>;

impl ObfuscatedStoreSeam for ObfuskierungUnit {
    /// Mintet die neue ID **synchron an der DB** (`randomblob`) und stagt das Zeilen-`INSERT` in die
    /// committende Transaktion; ein unerreichbarer Store (Schema fehlt / DB-Fehler) failt **hier**
    /// terminal als redigierte [`StoreUnavailable`]. Der `INSERT` selbst committet mit Ack und
    /// `Obfuscated`-Publish oder gar nicht.
    fn save(&self, booking: &ObfuscatedBooking) -> Result<String, StoreUnavailable> {
        self.store_dto(booking)
    }
}

/// Das Präfix, das dieser Store **jeder** eigenen Diagnose voranstellt — die eine Quelle für den
/// Text, den [`store_unavailable`] setzt und den der Leak-Test **positiv** festnagelt
/// ([Issue 0079](../../../../docs/issues/0079-leak-tests-auf-das-diagnose-praefix-festnageln.md)):
/// fiele die durchgereichte Diagnose weg oder käme sie aus einer anderen Quelle, verschwände es aus
/// der Meldung und der Test würde rot.
const UNAVAILABLE_PREFIX: &str = "obfuskiert store unavailable";

/// Eine redigierte [`StoreUnavailable`] aus io-eigener Diagnose (Zugriffs-Mechanik, nie ein
/// Zell-/Feldwert — der Seam-Vertrag verlangt die Redaktion).
fn store_unavailable(message: &str) -> StoreUnavailable {
    StoreUnavailable {
        message: format!("{UNAVAILABLE_PREFIX}: {message}"),
    }
}

/// Die abgeflachte [`ObfuscatedBooking`] in ihrer besitzenden Speicher-Form — **ohne** ihre ID und
/// darum **nicht** schreibbar. Der einzige Ort, an dem dieser Store die Adapter-DTO anfasst; alles
/// Fehlbare (die u64→i64-Deutung der Betrags-Magnitude) ist danach entschieden.
///
/// `pub` aus Sichtbarkeits-Zwang (sie steht als `Prepared` in der Signatur eines öffentlichen
/// Trait-`impl`, `private_interfaces`), **nicht** als API: das Modul `store` ist privat, also bleibt
/// sie außerhalb von `io` nicht konstruierbar und nicht lesbar (benennbar ist sie als
/// `<ObfuskiertBookings as StoreSpec>::Prepared` schon — nur eben inert).
pub struct BookingFields {
    source_id: String,
    ordering_account: String,
    counter_account: Option<String>,
    amount_minor_units: i64,
    currency: String,
    direction: String,
    recipient: RecipientRow,
    booking_day: String,
    value_date: String,
    purpose: Option<String>,
    booking_text: Option<String>,
    seed: i64,
}

/// Der flache Empfänger-Teil (der Summentyp [`Recipient`] auf vier Spalten aufgelöst): `kind` trägt
/// die Diskriminante (`CHECK`-erzwungen), die drei optionalen Spalten je nach Variante. Die
/// `creditor_id` einer Firma bleibt der öffentliche **Klartext**-Anker (ADR-0010).
struct RecipientRow {
    kind: &'static str,
    name: Option<String>,
    creditor_id: Option<String>,
    pseudonym: Option<String>,
}

impl From<&Recipient> for RecipientRow {
    fn from(recipient: &Recipient) -> Self {
        match recipient {
            Recipient::Company { name, creditor_id } => RecipientRow {
                kind: "company",
                name: name.as_ref().map(|text| text.as_str().to_string()),
                creditor_id: Some(creditor_id.clone()),
                pseudonym: None,
            },
            Recipient::Person { pseudonym } => RecipientRow {
                kind: "person",
                name: None,
                creditor_id: None,
                pseudonym: Some(pseudonym.as_str().to_string()),
            },
            Recipient::Absent => RecipientRow {
                kind: "absent",
                name: None,
                creditor_id: None,
                pseudonym: None,
            },
        }
    }
}

impl TryFrom<&ObfuscatedBooking> for BookingFields {
    type Error = StoreUnavailable;

    /// Fehlbar allein im Betrag: eine **Magnitude** jenseits `i64::MAX` passt nicht in die
    /// `INTEGER`-Spalte und failt terminal als redigierte [`StoreUnavailable`] (Speicher-Mechanik,
    /// nie ein Rohwert). Der `seed` dagegen ist ein **opaker `u64`** und wird **bitweise** als `i64`
    /// umgedeutet (`u64::cast_signed`, Rückweg `i64::cast_unsigned`), damit ein Seed > `i64::MAX`
    /// verlustfrei durch die INTEGER-Spalte reist, statt terminal abgelehnt zu werden.
    fn try_from(booking: &ObfuscatedBooking) -> Result<Self, StoreUnavailable> {
        Ok(Self {
            source_id: booking.source_id.clone(),
            ordering_account: booking.ordering_account.as_str().to_string(),
            counter_account: booking
                .counter_account
                .as_ref()
                .map(|account| account.as_str().to_string()),
            amount_minor_units: i64::try_from(booking.amount_minor_units)
                .map_err(|_overflow| store_unavailable("amount out of range for storage"))?,
            currency: booking.currency.clone(),
            direction: booking.direction.clone(),
            recipient: RecipientRow::from(&booking.recipient),
            booking_day: booking.booking_day.clone(),
            value_date: booking.value_date.clone(),
            purpose: booking
                .purpose
                .as_ref()
                .map(|text| text.as_str().to_string()),
            booking_text: booking
                .booking_text
                .as_ref()
                .map(|text| text.as_str().to_string()),
            seed: booking.seed.cast_signed(),
        })
    }
}

/// Die SQL-fertige Zeile: die abgeflachten Felder **unter** ihrer DB-vergebenen ID (zugleich der
/// publizierte Zeiger). Sie reist in die gestagete Ablage-Closure und wird erst am Commit-Rand
/// geschrieben. Die ID ist erster Parameter von [`new`](BookingRow::new); dass es die eben an der DB
/// gemintete ist, trägt ihr Typ [`MintedId`] — eine von Hand gebaute Schein-ID ist hier ein
/// Compile-Fehler.
struct BookingRow {
    id: MintedId,
    fields: BookingFields,
}

impl BookingRow {
    const fn new(id: MintedId, fields: BookingFields) -> Self {
        Self { id, fields }
    }

    fn insert(&self, tx: &Transaction<'_>) -> rusqlite::Result<()> {
        let fields = &self.fields;
        tx.execute(
            INSERT_BOOKING,
            params![
                self.id,
                fields.source_id,
                fields.ordering_account,
                fields.counter_account,
                fields.amount_minor_units,
                fields.currency,
                fields.direction,
                fields.recipient.kind,
                fields.recipient.name,
                fields.recipient.creditor_id,
                fields.recipient.pseudonym,
                fields.booking_day,
                fields.value_date,
                fields.purpose,
                fields.booking_text,
                fields.seed,
            ],
        )
        .map(drop)
    }
}

/// Die **eine** Tabelle des Slice mit ihrem DDL und ihrem Ablage-`INSERT`. Anlegen,
/// Audit-Spalten-Nachrüstung und Preflight laufen alle über diese Liste — eine Tabelle, die eines
/// davon verpasst, ist gar nicht erst deklarierbar.
const TABLES: &[TableSpec] = &[TableSpec {
    name: "obfuskiert",
    ddl: SCHEMA,
    insert: INSERT_BOOKING,
}];

/// Das Schema der `obfuskiert`-Ergebnis-Tabelle: die abgeflachte obfuskierte Buchung unter der
/// DB-vergebenen ID. Sie trägt **nur Token** (`KONTO_…`/`PERSON_…`, obfuskierter Freitext); einzige
/// Klartext-Ausnahme ist die öffentliche `recipient_creditor_id` (Analyse-Anker, ADR-0010). Die
/// Quell-Buchung ist **nur per `source_id`** referenziert (eine Referenz-per-ID auf
/// `einlesen_lines.id`, **kein** DB-`FOREIGN KEY`-Constraint — SQLite erzwingt FKs nur mit
/// aktiviertem `PRAGMA foreign_keys`, das die synchrone Outbox-Spine bewusst nicht schaltet), nie
/// eingebettet.
///
/// `direction` und `recipient_kind` sind enum-artig und per `CHECK`-Constraint erzwungen;
/// `created_at`/`updated_at` (Epoch-Millis) tragen den Audit-Zeitstempel jeder Zeile — beide
/// Konventionen aus `.rules/sql/schema-konventionen.md`. Die Tabelle ist append-only, daher fällt
/// `updated_at` mit `created_at` zusammen. Die **Referenz-Spalte `source_id` ist indiziert**
/// (`.rules/sql/queries-und-performance.md`), damit ein späterer Rückbezug auf die Quell-Zeile keinen
/// Full-Table-Scan erzwingt. Die `seed`-Spalte trägt den **opaken `u64`-Seed bitweise als `i64`
/// umgedeutet** (`u64::cast_signed`) — ein Seed > `i64::MAX` erscheint dadurch als negativer INTEGER,
/// reist aber verlustfrei (Rückweg `i64::cast_unsigned`), statt terminal abgelehnt zu werden.
const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS obfuskiert (
    id                    TEXT    PRIMARY KEY,
    source_id             TEXT    NOT NULL,
    ordering_account      TEXT    NOT NULL,
    counter_account       TEXT,
    amount_minor_units    INTEGER NOT NULL,
    currency              TEXT    NOT NULL,
    direction             TEXT    NOT NULL CHECK (direction IN ('incoming', 'outgoing')),
    recipient_kind        TEXT    NOT NULL CHECK (recipient_kind IN ('company', 'person', 'absent')),
    recipient_name        TEXT,
    recipient_creditor_id TEXT,
    recipient_pseudonym   TEXT,
    booking_day           TEXT    NOT NULL,
    value_date            TEXT    NOT NULL,
    purpose               TEXT,
    booking_text          TEXT,
    seed                  INTEGER NOT NULL,
    created_at            INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at            INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
);
CREATE INDEX IF NOT EXISTS obfuskiert_source_id_idx ON obfuskiert (source_id);";

// Die beiden `unixepoch('subsec')`-Aufrufe liefern **denselben** Zeitpunkt: SQLite berechnet die
// aktuelle Zeit **einmal pro Statement**, sodass created_at und updated_at bei diesem append-only
// Insert garantiert zusammenfallen — kein CTE nötig (siehe `einlesen`-Store).
const INSERT_BOOKING: &str = "\
INSERT INTO obfuskiert (
    id, source_id, ordering_account, counter_account, amount_minor_units, currency, direction,
    recipient_kind, recipient_name, recipient_creditor_id, recipient_pseudonym,
    booking_day, value_date, purpose, booking_text, seed,
    created_at, updated_at
) VALUES (
    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dispatcher;
    use crate::einlesen::EinlesenSqliteStore;
    use crate::obfuskierung::{
        CONSUMED_MESSAGE_TYPE, EinlesenRowSource, SeededObfuscator, consumer,
    };
    use crate::outbox::{OutboxError, OutboxMessage};
    use crate::pipeline_test_support::{
        SEED, Stage, assert_redacted_diagnosis, register_fresh, seed_through,
    };
    use crate::sqlite::SqliteOutbox;
    use application_kernel::PublishCommand;
    use obfuskierung_adapter::ObfuscatedMessage;

    /// Der `message_type` der stromabwärtigen `Obfuscated`-Nachricht — aus dem kanonischen
    /// `PublishCommand::MESSAGE_TYPE` des DTOs statt als String-Literal, damit eine Umbenennung nicht
    /// still am Test vorbeidriftet.
    const OBFUSCATED_MESSAGE_TYPE: &str = <ObfuscatedMessage as PublishCommand>::MESSAGE_TYPE;

    /// Die rohe Auftraggeber-IBAN der `gueltig.csv`-Fixture — sie darf in **keinem** obfuskiert-Token
    /// und in **keinem** Fehlergrund auftauchen (Datenschutz-Redaktion).
    const ROH_IBAN: &str = "DE89370400440532013000";

    /// Eine flach zurückgelesene `obfuskiert`-Zeile (nur die verwerteten Spalten) — genug, um
    /// Feld-Treue und die Datenschutz-Invariante am realen Bestand zu prüfen, ohne die
    /// crate-privaten Adapter-DTOs (`AccountId`/`Pseudonym`) rekonstruieren zu müssen.
    #[derive(Debug)]
    struct StoredBooking {
        id: String,
        source_id: String,
        ordering_account: String,
        counter_account: Option<String>,
        amount_minor_units: i64,
        currency: String,
        direction: String,
        recipient_kind: String,
        recipient_name: Option<String>,
        recipient_creditor_id: Option<String>,
        booking_day: String,
        seed: u64,
    }

    /// Liest die abgelegten obfuskiert-Zeilen (nur für Tests). Der reale Lesepfad des Folge-Slice
    /// (`fenstern`) liegt hinter dessen eigenem Seam (`ObfuskiertWindowSource`, Issue 0057).
    fn stored_bookings(store: &ObfuskierungSqliteStore) -> rusqlite::Result<Vec<StoredBooking>> {
        store.with_connection(|conn| {
            let mut statement = conn.prepare(
                "SELECT id, source_id, ordering_account, counter_account, amount_minor_units, \
                 currency, direction, recipient_kind, recipient_name, recipient_creditor_id, \
                 booking_day, seed FROM obfuskiert ORDER BY rowid;",
            )?;
            let rows = statement.query_map([], |row| {
                Ok(StoredBooking {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    ordering_account: row.get(2)?,
                    counter_account: row.get(3)?,
                    amount_minor_units: row.get(4)?,
                    currency: row.get(5)?,
                    direction: row.get(6)?,
                    recipient_kind: row.get(7)?,
                    recipient_name: row.get(8)?,
                    recipient_creditor_id: row.get(9)?,
                    booking_day: row.get(10)?,
                    // Rückweg der bitweisen Umdeutung: der als `i64` abgelegte Seed kommt
                    // verlustfrei als `u64` zurück (Gegenstück zu `u64::cast_signed` beim Schreiben).
                    seed: row.get::<_, i64>(11)?.cast_unsigned(),
                })
            })?;
            rows.collect()
        })
    }

    /// Die `partition_key`s der Nachrichten eines Typs in Einreih-Reihenfolge (die Zeiger).
    fn partition_keys(outbox: &SqliteOutbox, message_type: &str) -> rusqlite::Result<Vec<String>> {
        outbox.with_connection(|conn| {
            let mut statement = conn.prepare(
                "SELECT partition_key FROM outbox WHERE message_type = ?1 ORDER BY rowid;",
            )?;
            let rows = statement.query_map([message_type], |row| row.get(0))?;
            rows.collect()
        })
    }

    /// Fährt die **reale** zweistufige Pipeline auf **einem** Rückgrat: erst die `einlesen`-Vorstufe
    /// über der CSV-Fixture (per [`seed_through`] — landet die reale `einlesen_lines`-Zeile und
    /// publiziert `Ingested`), dann der `obfuskierung`-Consumer mit dem **gegebenen** Store und Seed,
    /// der genau diese `Ingested`-Nachricht konsumiert. Gibt die Ausgänge des **zweiten**
    /// (obfuskierung-)Laufs zurück.
    fn run_pipeline(
        outbox: &SqliteOutbox,
        obfuskierung_store: ObfuskierungSqliteStore,
        fixture: &str,
        seed: u64,
    ) -> rusqlite::Result<Vec<(OutboxMessage, Result<(), OutboxError>)>> {
        // Vorstufe einlesen seeden: die reale einlesen-Zeile landet, `Ingested` wird publiziert.
        seed_through(outbox, Stage::Einlesen, fixture)?;

        // Stufe obfuskierung: der **gegebene** Store (ggf. bewusst unerreichbar) und Seed über der
        // offenen `Ingested`-Nachricht.
        let source = EinlesenRowSource::new(outbox.clone());
        let obfuscator = SeededObfuscator;
        let obfuskierung = consumer(&source, &obfuscator, seed, obfuskierung_store);
        let mut dispatcher = Dispatcher::new();
        register_fresh(
            &mut dispatcher,
            CONSUMED_MESSAGE_TYPE,
            Box::new(obfuskierung),
        );
        outbox.deliver_open(&dispatcher)
    }

    #[expect(
        clippy::panic_in_result_fn,
        clippy::panic,
        reason = "Test-Helfer: assert!/panic! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_reale_zeile_wird_obfuskiert_abgelegt_und_publiziert_einen_zeiger()
    -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = ObfuskierungSqliteStore::new(outbox.clone())?;

        let outcomes = run_pipeline(&outbox, store.clone(), "gueltig.csv", SEED)?;

        // Die `Ingested`-Nachricht ist atomar verarbeitet (geackt) — genau ein Ok-Ausgang.
        assert!(
            matches!(outcomes.as_slice(), [(_, Ok(()))]),
            "die Ingested-Nachricht muss atomar verarbeitet werden: {outcomes:?}"
        );

        let bookings = stored_bookings(&store)?;
        let einlesen_ids = partition_keys(&outbox, CONSUMED_MESSAGE_TYPE)?;
        let buchung = match bookings.as_slice() {
            [buchung] => buchung,
            other => panic!("genau eine obfuskierte Buchung erwartet: {other:?}"),
        };

        // Uniformes ID-Handoff: source_id zeigt auf die reale einlesen-Zeile (deren ID = der von
        // einlesen publizierte `Ingested`-Zeiger).
        assert_eq!(
            einlesen_ids.as_slice(),
            std::slice::from_ref(&buchung.source_id),
            "source_id muss die reale einlesen-Zeilen-ID sein"
        );
        // Pass-Through-Felder feld-treu (EDEKA-Zeile: -50,00 EUR am 2024-06-13, ausgehend).
        assert_eq!(
            buchung.amount_minor_units, 5000,
            "Betrag verbatim durchgereicht"
        );
        assert_eq!(buchung.currency, "EUR", "Währung verbatim durchgereicht");
        assert_eq!(
            buchung.direction, "outgoing",
            "Richtung-Token verbatim durchgereicht"
        );
        assert_eq!(
            buchung.booking_day, "2024-06-13",
            "Buchungstag als ISO-Textstamp"
        );
        assert_eq!(buchung.seed, SEED, "der Seed reist mit");
        // Empfänger **mit** Gläubiger-ID → Firma: die öffentliche `creditor_id` bleibt der
        // Klartext-Analyse-Anker (ADR-0010). Der Firmenname reist als `ObfuscatedText` (best-effort,
        // heute nur trimmend — die Scrubbing-Regeln sind vertagt), also **nicht** strukturell
        // getokenisiert; die harte Datenschutz-Zusage prüft dieser Test an den IBAN-Token unten.
        assert_eq!(
            buchung.recipient_kind, "company",
            "mit Gläubiger-ID → Firma"
        );
        assert_eq!(
            buchung.recipient_creditor_id.as_deref(),
            Some("DE98ZZZ09999999999"),
            "die Gläubiger-ID bleibt der öffentliche Klartext-Anker"
        );
        assert!(
            buchung.recipient_name.is_some(),
            "der Firmenname reist als obfuskierter Freitext mit: {:?}",
            buchung.recipient_name
        );

        // Datenschutz: die Konto-Token sind `KONTO_…` und tragen die rohe IBAN **nicht**.
        assert!(
            buchung.ordering_account.starts_with("KONTO_")
                && !buchung.ordering_account.contains(ROH_IBAN),
            "das Auftraggeber-Token ist obfuskiert, ohne die rohe IBAN: {}",
            buchung.ordering_account
        );
        assert!(
            buchung
                .counter_account
                .as_deref()
                .is_some_and(|token| token.starts_with("KONTO_")),
            "das Gegenkonto-Token ist obfuskiert: {:?}",
            buchung.counter_account
        );

        // Tabelle = Wahrheit, Message = Zeiger: genau eine `Obfuscated`-Nachricht, deren Zeiger die
        // **neue** obfuskiert-ID ist — atomar in derselben Transaktion committet.
        assert_eq!(
            partition_keys(&outbox, OBFUSCATED_MESSAGE_TYPE)?,
            std::slice::from_ref(&buchung.id),
            "genau eine Obfuscated-Nachricht, deren partition_key die neue obfuskiert-ID ist"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        clippy::panic,
        reason = "Test-Helfer: assert!/panic! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ein_seed_jenseits_von_i64_max_wird_verlustfrei_round_trip_gespeichert()
    -> rusqlite::Result<()> {
        // Regression (Copilot-Review #55): der Seed ist ein opaker `u64`. Ein Wert > `i64::MAX`
        // (hier `u64::MAX`) muss **bitweise** durch die INTEGER-Spalte reisen und identisch
        // zurückkommen — `u64::cast_signed` beim Schreiben, `i64::cast_unsigned` beim Lesen. Das
        // frühere `i64::try_from` hätte ihn terminal als `StoreUnavailable` abgelehnt.
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = ObfuskierungSqliteStore::new(outbox.clone())?;

        let outcomes = run_pipeline(&outbox, store.clone(), "gueltig.csv", u64::MAX)?;
        assert!(
            matches!(outcomes.as_slice(), [(_, Ok(()))]),
            "der Seed jenseits von i64::MAX darf die Ablage nicht scheitern lassen: {outcomes:?}"
        );

        let bookings = stored_bookings(&store)?;
        let buchung = match bookings.as_slice() {
            [buchung] => buchung,
            other => panic!("genau eine obfuskierte Buchung erwartet: {other:?}"),
        };
        assert_eq!(
            buchung.seed,
            u64::MAX,
            "ein Seed jenseits von i64::MAX reist verlustfrei durch die INTEGER-Spalte"
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
        let store = ObfuskierungSqliteStore::without_schema(outbox.clone());

        let outcomes = run_pipeline(&outbox, store, "gueltig.csv", SEED)?;

        // Terminal (Warning), redigiert — beidseitig festgenagelt (Issue 0079): der Grund trägt das
        // Präfix **dieses** Stores (positiv, also ist die durchgereichte Diagnose wirklich da und
        // stammt aus dem Seam) und keine rohe IBAN, keinen Zellwert (negativ).
        assert_redacted_diagnosis(&outcomes, UNAVAILABLE_PREFIX, ROH_IBAN);
        // … und **nichts** ist stromabwärts publiziert (kein Dead-Letter/Retry, keine halbe Kette).
        assert!(
            partition_keys(&outbox, OBFUSCATED_MESSAGE_TYPE)?.is_empty(),
            "ein terminaler Store-Fehler darf nichts publizieren"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_fehlende_quell_zeile_failt_terminal_und_redigiert() -> rusqlite::Result<()> {
        // Der obfuskierung-Store steht, aber es gibt **keine** einlesen-Zeile: die `Ingested`-Nachricht
        // zeigt ins Leere. Die Tabelle **antwortet** hier, sie hat nur nichts zu dieser ID — der
        // Lese-Seam liefert also `Ok(None)` → terminaler, redigierter NotFound. Das ist der Kontrast
        // zum `Err`-Ausgang (Beschaffung misslungen), den `source.rs`
        // (`read_ohne_einlesen_tabelle_failt_terminal`) festnagelt: eine kaputte Datenbank meldet
        // seit Issue 0078 nicht mehr „nicht gefunden".
        let outbox = SqliteOutbox::open_in_memory()?;
        let store = ObfuskierungSqliteStore::new(outbox.clone())?;
        // Die einlesen-Tabelle existiert (für den Lese-Seam), ist aber leer.
        EinlesenSqliteStore::new(outbox.clone())?;

        let source = EinlesenRowSource::new(outbox.clone());
        let obfuscator = SeededObfuscator;
        let obfuskierung = consumer(&source, &obfuscator, SEED, store.clone());
        let mut dispatcher = Dispatcher::new();
        assert!(
            dispatcher
                .register(CONSUMED_MESSAGE_TYPE, Box::new(obfuskierung))
                .is_none(),
            "frische Registrierung darf keinen Consumer verdrängen"
        );
        // Ein `Ingested`-Zeiger auf eine ID, zu der keine einlesen-Zeile existiert.
        outbox.enqueue(CONSUMED_MESSAGE_TYPE, "es-gibt-mich-nicht")?;

        let outcomes = outbox.deliver_open(&dispatcher)?;

        // Terminal (Warning), redigiert: der Grund trägt die opake (Fantasie-)ID, keinen Rohwert.
        assert!(
            matches!(
                outcomes.as_slice(),
                [(_, Err(OutboxError::Warning { reason }))] if !reason.contains(ROH_IBAN)
            ),
            "eine fehlende Quell-Zeile muss terminal + redigiert failen: {outcomes:?}"
        );
        assert!(
            stored_bookings(&store)?.is_empty(),
            "eine fehlende Quell-Zeile darf nichts ablegen"
        );
        assert!(
            partition_keys(&outbox, OBFUSCATED_MESSAGE_TYPE)?.is_empty(),
            "eine fehlende Quell-Zeile darf nichts publizieren"
        );
        Ok(())
    }
}
