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
//! [`FensternSqliteStore`] ist die [`UnitFactory`](crate::UnitFactory) des Slice — je
//! Nachricht öffnet es eine [`FensternUnit`] über dem generischen [`SliceUnit`]. Deren
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
//! **Seit [Issue 0076](../../../../docs/issues/0076-fenstern-migrieren-zwei-tabellen-fall.md) trägt
//! dieser Store nur noch das Slice-Spezifische** ([ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md)):
//! sein [`StoreSpec`]-`impl` (Schema + Inserts + Flatten + Row-mit-ID) und die dünne Seam-`impl`, die
//! [`SliceUnit::store_dto`](crate::sqlite::SliceUnit) aufruft. Store-Struct, `new`, Unit-Newtype,
//! Preflight/Minten, die [`UnitFactory`](crate::UnitFactory) und der Outbox-Passthrough leben
//! generisch in [`SliceStore`]/[`SliceUnit`] — die Unit-Quelle seit dem `auswerten`-Capstone
//! ([Issue 0077](../../../../docs/issues/0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md)),
//! der die vier strukturgleichen `UnitFactory`-Traits zu **einem** kollabiert hat. Er ist
//! der **Mehr-Tabellen-Fall** der Abstraktion: **beide** Tabellen stehen mit DDL und Ablage-Statement
//! in **einer** [`TableSpec`]-Liste (also legt der Schema-Besitzer beide an und der Preflight probt **beide**
//! Statements), und [`stage`](StoreSpec::stage) liefert
//! **eine** Schreibung, die Kopf- und Detail-Zeilen positionsgetreu in dieselbe Transaktion schreibt.
//! Der Seam-Vertrag Richtung Slice ist davon **unberührt**
//! ([ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)).
//!
//! **Fehlbar + terminal:** ein unerreichbarer Store (Schema fehlt) failt **synchron** in `save` als
//! [`StoreUnavailable`] (→ Domänen-`WindowError::StorageUnavailable`), terminal. Kein Dead-Letter,
//! kein Retry im Slice.

use fenstern_adapter::{AnalysisWindowRow, AnalysisWindowStoreSeam, StoreUnavailable};
use rusqlite::{Transaction, params};

use crate::sqlite::{MintedId, SliceStore, SliceUnit, StagedRow, StoreSpec, TableSpec};

/// Die `fenstern`-Persistenz als [`StoreSpec`]: der **Zwei-Tabellen-Fall** — Kopf
/// (`auswertungsfenster`) und Detail (`auswertungsfenster_buchungen`) je mit ihrem `INSERT`, und als
/// fehlbare Umdeutung die Ordenbarkeits-Absicherung über die Zahl der Buchungen. Der Marker-Typ
/// parametrisiert [`SliceStore`]/[`SliceUnit`] auf diesen Slice.
pub struct FensternWindows;

impl StoreSpec for FensternWindows {
    type Dto = AnalysisWindowRow;
    /// Das abgeflachte Fenster ohne seine ID — schreibbar wird erst die `WindowRow`, die `stage`
    /// daraus **mit** der geminteten ID baut (Kopf **und** Detail tragen sie).
    type Prepared = WindowFields;
    type Error = StoreUnavailable;

    fn tables() -> &'static [TableSpec] {
        TABLES
    }

    fn flatten(window: &AnalysisWindowRow) -> Result<WindowFields, StoreUnavailable> {
        WindowFields::try_from(window)
    }

    /// Baut **eine** Schreibung über beide Tabellen: erst die Kopf-Zeile unter der frisch geminteten
    /// `id`, dann positionsgetreu ihre Detail-Zeilen unter derselben `id`.
    fn stage(fields: WindowFields, id: MintedId) -> StagedRow {
        let row = WindowRow::new(id, fields);
        Box::new(move |tx| row.insert(tx))
    }

    /// Reicht die `rusqlite`-Diagnose eines misslungenen Store-Zugriffs durch (fehlendes Schema,
    /// DB-Fehler) — Mechanik, nie ein Zell-/Feldwert.
    fn unavailable(error: rusqlite::Error) -> StoreUnavailable {
        store_unavailable(&error.to_string())
    }
}

/// Der reale `fenstern`-Ergebnis-Store: der generische [`SliceStore`] über
/// [`FensternWindows`] und damit die [`UnitFactory`](crate::UnitFactory) des
/// Slice. `Clone` teilt dasselbe Rückgrat (`Rc` im Innern).
pub type FensternSqliteStore = SliceStore<FensternWindows>;

/// Eine `fenstern`-Arbeitseinheit: der gestagete Ergebnis-Speicher (`AnalysisWindowStoreSeam`)
/// **und** die Outbox (`OutboxPublisher`) in einem, atomar committbar (`UnitOfWork`).
pub type FensternUnit = SliceUnit<FensternWindows>;

impl AnalysisWindowStoreSeam for FensternUnit {
    /// Mintet die neue ID **synchron an der DB** (`randomblob`) und stagt das Fenster- und die
    /// Buchungs-`INSERT`s in die committende Transaktion; ein unerreichbarer Store (Schema fehlt /
    /// DB-Fehler) failt **hier** terminal als redigierte [`StoreUnavailable`]. Der Preflight probt
    /// **beide** Ablage-Statements (Kopf **und** Detail — beide stehen in `TABLES`): fehlt auch nur
    /// eine der beiden Tabellen, failt es hier terminal statt erst als Commit-Retry.
    fn save(&self, window: &AnalysisWindowRow) -> Result<String, StoreUnavailable> {
        self.store_dto(window)
    }
}

/// Eine redigierte [`StoreUnavailable`] aus io-eigener Diagnose (Zugriffs-Mechanik, nie ein
/// Zell-/Feldwert — der Seam-Vertrag verlangt die Redaktion).
fn store_unavailable(message: &str) -> StoreUnavailable {
    StoreUnavailable {
        message: format!("auswertungsfenster-Store nicht erreichbar: {message}"),
    }
}

/// Das abgeflachte [`AnalysisWindowRow`] in seiner besitzenden Speicher-Form — **ohne** seine ID und
/// darum **nicht** schreibbar. Der einzige Ort, an dem dieser Store die Adapter-DTO anfasst; alles
/// Fehlbare (die Ordenbarkeits-Absicherung über die Zahl der Buchungen) ist danach entschieden.
///
/// `pub` aus Sichtbarkeits-Zwang (sie steht als `Prepared` in der Signatur eines öffentlichen
/// Trait-`impl`, `private_interfaces`), **nicht** als API: das Modul `store` ist privat, also bleibt
/// sie außerhalb von `io` nicht konstruierbar und nicht lesbar (benennbar ist sie als
/// `<FensternWindows as StoreSpec>::Prepared` schon — nur eben inert).
pub struct WindowFields {
    run_id: String,
    from_day: i64,
    to_day: i64,
    currency: String,
    /// Die Buchungs-Fremdschlüssel in ihrer Reihenfolge — **sie** ist die Position; die
    /// Positionszahl entsteht erst beim Schreiben, kann der Reihenfolge also nicht widersprechen.
    bookings: Vec<String>,
}

impl TryFrom<&AnalysisWindowRow> for WindowFields {
    type Error = StoreUnavailable;

    /// Übernimmt die Buchungs-Fremdschlüssel in ihrer **Reihenfolge** — die *ist* die Position, darum
    /// trägt keine Buchung ihre Positionszahl daneben (sie könnte ihr sonst widersprechen);
    /// nummeriert wird erst beim Schreiben ([`WindowRow::insert`]).
    ///
    /// Fehlbar allein im vorangestellten Wächter: dass die **Zahl** der Buchungen in den
    /// `i64`-Positionszähler passt, ist die eine reine Speicher-Mechanik-Absicherung (ein Fenster mit
    /// mehr als `i64::MAX` Buchungen ist unrealistisch). Sie failt terminal als [`StoreUnavailable`],
    /// nie mit einem Rohwert — durch [`store_unavailable`], den einen redigierenden Konstruktor
    /// dieses Moduls. Läuft im fehlbaren [`flatten`](StoreSpec::flatten) und damit **vor** dem
    /// Minten, damit [`WindowRow::new`] die schreibbare Row **in einem Zug mit** der ID bauen kann.
    fn try_from(window: &AnalysisWindowRow) -> Result<Self, StoreUnavailable> {
        i64::try_from(window.booking_ids.len()).map_err(|_overflow| {
            store_unavailable("Fenster trägt mehr Buchungen als sich ordnen lassen")
        })?;
        Ok(Self {
            run_id: window.run.clone(),
            from_day: window.from,
            to_day: window.to,
            currency: window.currency.clone(),
            bookings: window.booking_ids.clone(),
        })
    }
}

/// Die SQL-fertigen Zeilen **beider** Tabellen: die abgeflachten Felder **unter** ihrer
/// DB-vergebenen ID (zugleich der publizierte Zeiger), die Kopf und Detail gemeinsam tragen. Sie
/// reist in die gestagete Ablage-Closure und wird erst am Commit-Rand geschrieben. Die ID ist erster
/// Parameter von [`new`](WindowRow::new); dass es die eben an der DB gemintete ist, trägt ihr Typ
/// [`MintedId`] — eine von Hand gebaute Schein-ID, die Kopf und Detail unter einem
/// Schein-Primärschlüssel zusammenbände, ist hier ein Compile-Fehler.
struct WindowRow {
    id: MintedId,
    fields: WindowFields,
}

impl WindowRow {
    /// Baut die schreibbare Row unter der frisch geminteten DB-`id` (zugleich der publizierte Zeiger).
    const fn new(id: MintedId, fields: WindowFields) -> Self {
        Self { id, fields }
    }

    /// Schreibt beide Tabellen in **einer** Transaktion: die Kopf-Zeile, dann positionsgetreu ihre
    /// Detail-Zeilen unter derselben ID. Die Positionszahl entsteht **hier**, aus der Reihenfolge
    /// selbst (`0i64..`), statt in den Feldern neben ihr zu liegen — sie kann ihr also nicht
    /// widersprechen; dass der Zähler reicht, hat der Wächter in `WindowFields::try_from` über die
    /// Zahl der Buchungen abgesichert.
    fn insert(&self, tx: &Transaction<'_>) -> rusqlite::Result<()> {
        let fields = &self.fields;
        tx.execute(
            INSERT_WINDOW,
            params![
                self.id,
                fields.run_id,
                fields.from_day,
                fields.to_day,
                fields.currency
            ],
        )?;
        (0i64..)
            .zip(fields.bookings.iter())
            .try_for_each(|(position, booking_id)| {
                tx.execute(INSERT_BOOKING, params![self.id, position, booking_id])
                    .map(drop)
            })
    }
}

/// Die **beiden** Tabellen des Slice, je mit ihrem DDL und ihrem Ablage-`INSERT` — der
/// Mehr-Tabellen-Fall der Abstraktion. Anlegen (durch den Schema-Besitzer, [`schema`](crate::schema))
/// und Preflight laufen beide über diese Liste, also gilt der Preflight-Beweis für **beide**: eine Tabelle, die eines davon
/// verpasst, ist gar nicht erst deklarierbar.
const TABLES: &[TableSpec] = &[
    TableSpec {
        name: "auswertungsfenster",
        ddl: SCHEMA_WINDOWS,
        insert: INSERT_WINDOW,
    },
    TableSpec {
        name: "auswertungsfenster_buchungen",
        ddl: SCHEMA_BOOKINGS,
        insert: INSERT_BOOKING,
    },
];

/// Das Schema der **Kopf**-Tabelle: `auswertungsfenster` trägt den **Lauf**, die Spanne (als
/// `i64`-Tag-Stamps) und die eine Währung unter der DB-vergebenen ID.
///
/// **`run_id`** ([ADR-0016](../../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)):
/// jedes Fenster gehört zu genau **einem** Lauf — jeder Lauf paginiert die ganze Historie neu, also
/// lägen ohne sie die Fenster aller Läufe ununterscheidbar nebeneinander. Die fenster-übergreifende
/// Abfrage von `auswerten` scopt über diese Spalte; darum ist sie indiziert. Die Auswertungen eines
/// Fensters gehören über ihre `window_id` demselben Lauf an — eine eigene Lauf-Spalte bräuchten sie
/// nicht. `created_at`/`updated_at` (Epoch-Millis) tragen den
/// Audit-Zeitstempel; die Tabelle ist append-only, darum fällt `updated_at` mit `created_at` zusammen.
const SCHEMA_WINDOWS: &str = "\
CREATE TABLE IF NOT EXISTS auswertungsfenster (
    id         TEXT    PRIMARY KEY,
    run_id     TEXT    NOT NULL,
    from_day   INTEGER NOT NULL,
    to_day     INTEGER NOT NULL,
    currency   TEXT    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
);
CREATE INDEX IF NOT EXISTS auswertungsfenster_run_id_idx
    ON auswertungsfenster (run_id);";

/// Das Schema der **Detail**-Tabelle: `auswertungsfenster_buchungen` trägt die Fremdschlüssel auf die
/// obfuskierten Zeilen, **positionsgetreu** (`window_id`, `position`) — **kein** DB-`FOREIGN
/// KEY`-Constraint (SQLite erzwingt FKs nur mit aktiviertem `PRAGMA foreign_keys`, das die synchrone
/// Outbox-Spine bewusst nicht schaltet), aber die **Referenz-Spalte `window_id` ist indiziert**
/// (`.rules/sql/queries-und-performance.md`). Audit-Zeitstempel wie bei der Kopf-Tabelle.
const SCHEMA_BOOKINGS: &str = "\
CREATE TABLE IF NOT EXISTS auswertungsfenster_buchungen (
    window_id  TEXT    NOT NULL,
    position   INTEGER NOT NULL,
    booking_id TEXT    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    PRIMARY KEY (window_id, position)
);
CREATE INDEX IF NOT EXISTS auswertungsfenster_buchungen_window_id_idx
    ON auswertungsfenster_buchungen (window_id);";

const INSERT_WINDOW: &str = "\
INSERT INTO auswertungsfenster (id, run_id, from_day, to_day, currency, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

const INSERT_BOOKING: &str = "\
INSERT INTO auswertungsfenster_buchungen (window_id, position, booking_id, created_at, updated_at)
VALUES (?1, ?2, ?3,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use application_kernel::PublishCommand;
    use fenstern_adapter::{HistoryCompleteMessage, WindowClosedMessage};
    use obfuskierung_adapter::ObfuscatedMessage;
    use rusqlite::params;

    use super::*;
    use crate::Dispatcher;
    use crate::fenstern::{
        CONSUMED_MESSAGE_TYPE as FENSTERN_CONSUMED, ObfuskiertWindowSource, START_MESSAGE_TYPE,
        consumer as fenstern_consumer, start_consumer,
    };
    use crate::obfuskierung::ObfuskierungSqliteStore;
    use crate::outbox::{OutboxError, OutboxMessage};
    use crate::pipeline_test_support::{
        CapturingAck, Stage, drain, register_fresh, seed_through, stamp,
    };
    use crate::sqlite::{Delivery, SqliteOutbox};

    /// Die `message_type`s der stromabwärtigen Nachrichten — aus den kanonischen
    /// `PublishCommand::MESSAGE_TYPE`s der DTOs statt als String-Literal, damit eine Umbenennung
    /// nicht still an den Tests vorbeidriftet.
    const OBFUSCATED_MESSAGE_TYPE: &str = <ObfuscatedMessage as PublishCommand>::MESSAGE_TYPE;
    const WINDOW_CLOSED_MESSAGE_TYPE: &str = <WindowClosedMessage as PublishCommand>::MESSAGE_TYPE;
    const HISTORY_COMPLETE_MESSAGE_TYPE: &str =
        <HistoryCompleteMessage as PublishCommand>::MESSAGE_TYPE;

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

    /// Die `partition_key`s aller publizierten Nachrichten eines Typs (nur für Tests) — belegt, dass
    /// ein terminaler Fehler **keinen** Zeiger stromabwärts hinterlässt (ADR-0012, keine halbe Kette).
    fn published(outbox: &SqliteOutbox, message_type: &str) -> rusqlite::Result<Vec<String>> {
        outbox.with_connection(|conn| {
            let mut statement = conn.prepare(
                "SELECT partition_key FROM outbox WHERE message_type = ?1 ORDER BY rowid;",
            )?;
            statement
                .query_map(params![message_type], |row| row.get(0))?
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

    /// Fährt den `fenstern`-Slice über dem **gegebenen** Store einmal durch: der Anstoß
    /// (`StartAnalysis`) und der sich selbst fortsetzende Fenster-Schritt hängen am realen Lese-Seam,
    /// die stromabwärtigen Zeiger räumt der gegebene [`CapturingAck`] ab (sonst konvergiert die
    /// Broker-Schleife nie) und protokolliert sie zugleich. Gibt die Ausgänge des ganzen Laufs
    /// zurück. Bündelt die sonst je Test wiederholte Consumer-Verdrahtung an **einer** Stelle — wie
    /// `run_pipeline` im `obfuskierung`-Store, dessen Muster dieses Modul repliziert —, sodass jeder
    /// Test nur noch seinen eigenen Arrange-Zustand herstellt und prüft.
    fn run_fenstern(
        outbox: &SqliteOutbox,
        store: FensternSqliteStore,
        window_days: NonZeroU32,
        ack: &CapturingAck,
    ) -> rusqlite::Result<Vec<(OutboxMessage, Delivery)>> {
        let source = ObfuskiertWindowSource::new(outbox.clone());
        let mut dispatcher = Dispatcher::new();
        register_fresh(
            &mut dispatcher,
            OBFUSCATED_MESSAGE_TYPE,
            Box::new(ack.clone()),
        );
        register_fresh(
            &mut dispatcher,
            WINDOW_CLOSED_MESSAGE_TYPE,
            Box::new(ack.clone()),
        );
        register_fresh(
            &mut dispatcher,
            HISTORY_COMPLETE_MESSAGE_TYPE,
            Box::new(ack.clone()),
        );
        register_fresh(
            &mut dispatcher,
            START_MESSAGE_TYPE,
            Box::new(start_consumer(&source, store.clone())),
        );
        register_fresh(
            &mut dispatcher,
            FENSTERN_CONSUMED,
            Box::new(fenstern_consumer(window_days, &source, store)),
        );

        outbox.enqueue(START_MESSAGE_TYPE, "lauf-1")?;
        drain(outbox, &dispatcher)
    }

    /// Die Standard-Seitengröße der Store-Tests: drei Tage je Fenster.
    fn drei_tage() -> NonZeroU32 {
        NonZeroU32::new(3).expect("Seitengröße ist nicht null")
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn mehrere_seiten_werden_real_paginiert_gespeichert_und_schliessen_mit_historie_vollstaendig()
    -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        seed_through(&outbox, Stage::Obfuskierung, "mehrseitig.csv")?;

        // Die vier realen, DB-geminteten obfuskiert-IDs — über den je CSV-Zeile eindeutigen Betrag
        // zugeordnet (A=20.06., B=19.06., C=15.06., D=10.06.).
        let id_a = obfuskiert_id_by_amount(&outbox, 1000)?;
        let id_b = obfuskiert_id_by_amount(&outbox, 2000)?;
        let id_c = obfuskiert_id_by_amount(&outbox, 3000)?;
        let id_d = obfuskiert_id_by_amount(&outbox, 4000)?;

        let store = FensternSqliteStore::with_own_tables(outbox.clone())?;
        let ack = CapturingAck::new(outbox.clone());
        let outcomes = run_fenstern(&outbox, store, drei_tage(), &ack)?;

        assert!(
            outcomes.iter().all(|(_, outcome)| outcome.is_processed()),
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

        let seen = ack.seen();
        let window_closed = seen
            .iter()
            .filter(|(typ, _)| typ == WINDOW_CLOSED_MESSAGE_TYPE)
            .count();
        assert_eq!(
            window_closed, 3,
            "je materialisiertem Fenster ein WindowClosed-Zeiger"
        );

        let history_complete: Vec<String> = seen
            .iter()
            .filter(|(typ, _)| typ == HISTORY_COMPLETE_MESSAGE_TYPE)
            .map(|(_, key)| key.clone())
            .collect();
        assert_eq!(
            history_complete,
            vec![format!(
                "{}:{}:lauf-1",
                stamp(2024, 6, 8),
                stamp(2024, 6, 20)
            )],
            "genau ein HistoryComplete mit der Lauf-Spanne (Boden der letzten Seite bis Lauf-Gipfel) \
             und dem Lauf"
        );
        // Jedes Fenster gehört dem Lauf, den der Anstoß eröffnet hat (ADR-0016).
        let fremde_laeufe: i64 = outbox.with_connection(|conn| {
            conn.query_row(
                "SELECT count(*) FROM auswertungsfenster WHERE run_id <> 'lauf-1';",
                [],
                |row| row.get(0),
            )
        })?;
        assert_eq!(
            fremde_laeufe, 0,
            "jedes Fenster trägt den Lauf des Anstoßes"
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
        ObfuskierungSqliteStore::with_own_tables(outbox.clone())?;

        let store = FensternSqliteStore::with_own_tables(outbox.clone())?;
        let ack = CapturingAck::new(outbox.clone());
        let outcomes = run_fenstern(&outbox, store, drei_tage(), &ack)?;

        assert!(
            matches!(outcomes.as_slice(), [(_, Delivery::Processed)]),
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
        // Bewusst **keine** obfuskiert-Tabelle angelegt — nur die eigenen des fenstern-Stores.
        let store = FensternSqliteStore::with_own_tables(outbox.clone())?;
        let ack = CapturingAck::new(outbox.clone());
        let outcomes = run_fenstern(&outbox, store, drei_tage(), &ack)?;

        assert!(
            matches!(
                outcomes.as_slice(),
                [(_, Delivery::DeadLettered(OutboxError::Warning { .. }))]
            ),
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
        seed_through(&outbox, Stage::Obfuskierung, "gueltig.csv")?;

        // Die Ergebnis-Tabellen fehlen (`new` legt kein Schema an) → unerreichbar.
        let store = FensternSqliteStore::new(outbox.clone());
        let ack = CapturingAck::new(outbox.clone());
        let outcomes = run_fenstern(&outbox, store, drei_tage(), &ack)?;

        // Der Grund trägt die redigierte io-Diagnose, nie eine rohe Buchungs-ID.
        assert!(
            matches!(
                outcomes.last(),
                Some((_, Delivery::DeadLettered(OutboxError::Warning { .. })))
            ),
            "ein unerreichbarer Fenster-Store muss den Fenster-Schritt terminal failen lassen: \
             {outcomes:?}"
        );
        // …und **ohne zu publizieren**: der terminale Store-Fehler hinterlässt weder einen
        // WindowClosed- noch einen HistoryComplete-Zeiger stromabwärts (ADR-0012, keine halbe Kette).
        assert!(
            published(&outbox, WINDOW_CLOSED_MESSAGE_TYPE)?.is_empty()
                && published(&outbox, HISTORY_COMPLETE_MESSAGE_TYPE)?.is_empty(),
            "ein terminaler Fenster-Store-Fehler darf keinen Zeiger publizieren: {outcomes:?}"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ein_fehlendes_detail_schema_failt_die_ablage_terminal_ohne_zu_publizieren()
    -> rusqlite::Result<()> {
        // Der Mehr-Tabellen-Beweis (Issue 0076, Kriterium 2): **nur** die Kopf-Tabelle steht, die
        // Detail-Tabelle fehlt. Weil der Preflight über **beide** `TableSpec::insert` aus `TABLES`
        // läuft, failt schon `save` terminal — nicht erst der Commit als Retry. Fiele die
        // Detail-Tabelle aus `TABLES` (oder würde ihr `insert` nicht mehr geprobt), wird dieser Test
        // rot, während der Ausfall-Test über den ganz schemalosen Store weiter grün bliebe.
        let outbox = SqliteOutbox::open_in_memory()?;
        seed_through(&outbox, Stage::Obfuskierung, "gueltig.csv")?;

        // **Nur** die Kopf-Tabelle anlegen; `new` legt die fehlende Detail-Tabelle nicht nach.
        outbox.with_connection(|conn| conn.execute_batch(SCHEMA_WINDOWS))?;
        let store = FensternSqliteStore::new(outbox.clone());
        let ack = CapturingAck::new(outbox.clone());
        let outcomes = run_fenstern(&outbox, store, drei_tage(), &ack)?;

        assert!(
            matches!(
                outcomes.last(),
                Some((_, Delivery::DeadLettered(OutboxError::Warning { .. })))
            ),
            "eine fehlende Detail-Tabelle muss den Fenster-Schritt terminal + redigiert failen \
             lassen: {outcomes:?}"
        );
        assert!(
            published(&outbox, WINDOW_CLOSED_MESSAGE_TYPE)?.is_empty()
                && published(&outbox, HISTORY_COMPLETE_MESSAGE_TYPE)?.is_empty(),
            "ein terminaler Fenster-Store-Fehler darf keinen Zeiger publizieren: {outcomes:?}"
        );
        // Auch die **Kopf**-Zeile darf nicht liegenbleiben: der Preflight failt vor dem Minten, also
        // committet keine der beiden Tabellen etwas (Kopf und Detail oder gar nichts).
        let kopf_zeilen: i64 = outbox.with_connection(|conn| {
            conn.query_row("SELECT COUNT(*) FROM auswertungsfenster;", [], |row| {
                row.get(0)
            })
        })?;
        assert_eq!(
            kopf_zeilen, 0,
            "ohne Detail-Tabelle darf auch keine Kopf-Zeile committen"
        );
        Ok(())
    }
}
