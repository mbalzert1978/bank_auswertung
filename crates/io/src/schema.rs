//! Der **Schema-Besitzer**
//! ([Issue 0084](../../../docs/issues/0084-schema-an-einem-ort-und-versionsstempel-als-migrationsachse.md)):
//! die **eine** Stelle, an der eine Datenbank ihr Schema bekommt — die Outbox **und** die Tabellen
//! aller vier Ergebnis-Stores —, und die Stelle, die die Datei mit der Schema-Version des Programms
//! **stempelt**.
//!
//! Vorher legte jeder Store sein Schema beim Konstruieren selbst an (`CREATE TABLE IF NOT EXISTS`).
//! Niemand besaß das Schema als Ganzes, und jede vorgefundene Datei stand ununterscheidbar auf
//! Version 0 — eine spätere Migration hätte keinen Anker gehabt. Jetzt gilt:
//!
//! - **Eine frische Datei** (kein Stempel, kein einziges Schema-Objekt) bekommt **alle** Tabellen und
//!   den Stempel [`SCHEMA_VERSION`] — in **einer** Transaktion, also ganz oder gar nicht.
//! - **Eine Datei mit derselben Version** wird benutzt, wie sie ist. Es wird nichts angelegt.
//! - **Jede andere Datei** wird abgewiesen, **terminal und redigiert** ([`VersionMismatch`]), bevor
//!   irgendetwas in sie geschrieben wird. Das schließt eine Datei **ohne** Stempel ein, die schon
//!   Tabellen trägt: sie stammt aus der Zeit vor diesem Stempel (Version 0), und ob ihr Schema dem
//!   heutigen gleicht, weiß niemand — die Nachrüstung, die das früher still versuchte, ist
//!   ersatzlos entfallen. Weiterzuschreiben hieße, auf einem fremden Schema zu raten.
//!
//! **Der Stempel ist SQLites eingebautes `PRAGMA user_version`** — ein Ganzzahl-Feld im Kopf der
//! Datei, transaktional wie jede andere Schreibung, ohne eigene Tabelle.
//!
//! **Wo eine Migration ansetzt: an der Leiter.** Die Versionen laufen über die eine Weiche `ensure`:
//! „frisch", „passt", der Sonderfall Version 1 (siehe unten) und die **Leiter** (`LADDER`). Jede
//! Sprosse hebt eine Datei um **genau eine** Version, mit fester Zielversion; `ensure` steigt von der
//! vorgefundenen Version Sprosse für Sprosse bis [`SCHEMA_VERSION`] und stempelt **einmal** am Ende.
//! Eine neue Migration ist eine neue Sprosse am Ende der Leiter, dazu [`SCHEMA_VERSION`] hochgezählt —
//! die DDL der Stores beschreibt dann das **neue** Schema, das eine frische Datei direkt bekommt. Ein
//! Test hält die Leiter lückenlos bis zur aktuellen Version.
//!
//! **Version 1** ([ADR-0016](../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md))
//! steht **neben** der Leiter, weil sie keine Stufe ist, sondern ein Neubau (`rebuild_unkeyed_v1`): sie
//! landet direkt auf der aktuellen Version. Die erste echte Migration war 1 → 2: `einlesen_lines` bekommt den Zeilen-Schlüssel (`row_key`,
//! `UNIQUE`), `auswertungsfenster` den Lauf (`run_id`). Eine Version-1-Datei **ohne** Bestand wird
//! umgebaut; eine **mit** Bestand wird abgewiesen — terminal, redigiert, unangetastet. Ihre Zeilen
//! lassen sich nicht nachträglich schlüsseln: der Schlüssel entsteht aus Spalten, die Version 1 nie
//! abgelegt hat (Kundenreferenz, Mandatsreferenz, BIC, …). Ein Ersatzschlüssel aus dem, was da ist,
//! träfe beim nächsten Import desselben Auszugs nie — der Bestand verdoppelte sich still, genau der
//! Fehler, den Version 2 behebt. Lieber laut eine frische Datei verlangen.
//!
//! **Version 2 → 3** ([Issue 0009](../../../docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md))
//! ist die erste Sprosse der Leiter und rein **additiv**: das Kategorie-Register (`learned_categories`)
//! samt seinem Index kommt hinzu, keine vorhandene Tabelle ändert sich. Eine Version-2-Datei wird darum
//! **mit** ihrem Bestand umgebaut — es geht nichts verloren, und danach trägt sie genau das Schema einer
//! frischen Datei.
//!
//! **Version 3 → 4** ([Issue 0085](../../../docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md),
//! [ADR-0017](../../../docs/adr/0017-dead-letter-als-status-spalte-der-outbox.md)) gibt der Outbox
//! ihre `status`-Spalte (`open`/`processed`/`dead_letter`) und den redigierten Aussonderungs-Grund.
//! Auch hier geht **mit** Bestand nichts verloren: jede Zeile weiß schon, wo sie steht —
//! `processed_on_utc` gesetzt heißt `processed`, sonst `open`. Die Tabelle wird umgebaut und ihr
//! Bestand übernommen, samt `rowid` (sie trägt die Zustell-Reihenfolge). Die Ziel-DDL ist in der
//! Sprosse **eingefroren** (`OUTBOX_VERSION_4`): ändert sich die Outbox später, bleibt diese Sprosse,
//! was sie ist, und die Änderung wird eine eigene.

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::auswerten::{AuswertenEvaluations, LEARNED_CATEGORIES_TABLE};
use crate::config::AppConfig;
use crate::einlesen::EinlesenLines;
use crate::fenstern::FensternWindows;
use crate::obfuskierung::ObfuskiertBookings;
use crate::sqlite::{OUTBOX_TABLE, SqliteOutbox, StoreSpec, TableSpec};

/// Die Schema-Version **dieses Programms** — der Wert, mit dem eine frisch angelegte Datei gestempelt
/// wird und den eine vorgefundene Datei tragen muss. Hochzählen nur zusammen mit einer Migration in
/// `ensure`.
pub const SCHEMA_VERSION: i64 = 4;

/// Öffnet die Datenbank-Datei unter dem konfigurierten Pfad als Rückgrat — der **produktive**
/// Öffnungspfad. Eine frische Datei bekommt Schema und Stempel, eine passend gestempelte wird
/// unverändert benutzt, jede andere abgewiesen (siehe Modul-Doku).
///
/// # Errors
///
/// [`SchemaError::VersionMismatch`], wenn die Datei eine andere Schema-Version trägt (oder ohne
/// Stempel schon Tabellen hat); [`SchemaError::Unavailable`], wenn sie sich nicht öffnen, lesen oder
/// anlegen lässt.
pub fn open(config: &AppConfig) -> Result<SqliteOutbox, SchemaError> {
    let conn = Connection::open(config.database_path())?;
    ensure(&conn)?;
    Ok(SqliteOutbox::from_connection(conn))
}

/// Öffnet ein **flüchtiges** In-Memory-Rückgrat mit dem vollen Schema — über **denselben** Besitzer
/// wie [`open`], nur ohne Datei. Für Tests und ephemere Läufe: die DB stirbt mit dem Prozess.
///
/// # Errors
///
/// [`SchemaError::Unavailable`], wenn SQLite das Schema nicht anlegen kann.
pub fn open_in_memory() -> Result<SqliteOutbox, SchemaError> {
    let conn = Connection::open_in_memory()?;
    ensure(&conn)?;
    Ok(SqliteOutbox::from_connection(conn))
}

/// Was die geöffnete Datei vorfindet: nichts (frisch) oder einen Stempel. Eine Datei ohne Stempel,
/// die schon Schema-Objekte trägt, ist `Version(0)` — nicht frisch.
enum Found {
    Empty,
    Version(i64),
}

/// Bringt die Datei auf [`SCHEMA_VERSION`] oder weist sie ab — die eine Weiche über alle Versionen.
/// Hier setzt eine Migration als zusätzlicher Arm an (siehe Modul-Doku).
///
/// Prüfen und Anlegen laufen in **einer** `IMMEDIATE`-Transaktion: die Schreibsperre steht, bevor
/// gelesen wird, also kann zwischen „frisch" und „anlegen" kein zweiter Prozess dieselbe Datei
/// anlegen. Ein abgewiesener Fall rollt zurück — geschrieben wurde da ohnehin nichts.
fn ensure(conn: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    match found(&transaction)? {
        Found::Empty => create(&transaction)?,
        Found::Version(SCHEMA_VERSION) => {}
        Found::Version(1) => rebuild_unkeyed_v1(&transaction)?,
        Found::Version(found) if LADDER.iter().any(|rung| rung.from == found) => {
            climb(&transaction, found)?;
        }
        Found::Version(found) => {
            return Err(SchemaError::VersionMismatch(VersionMismatch {
                found,
                expected: SCHEMA_VERSION,
                cause: MismatchCause::UnknownVersion,
            }));
        }
    }
    transaction.commit()?;
    Ok(())
}

fn found(conn: &Connection) -> rusqlite::Result<Found> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let objects: i64 =
        conn.query_row("SELECT count(*) FROM sqlite_master;", [], |row| row.get(0))?;
    Ok(match (version, objects) {
        (0, 0) => Found::Empty,
        (version, _) => Found::Version(version),
    })
}

/// Legt **alle** Tabellen an und stempelt die Datei — innerhalb der Transaktion von `ensure`:
/// scheitert ein Schritt, bleibt die Datei leer und ungestempelt, also beim nächsten Öffnen wieder
/// „frisch".
fn create(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    tables().try_for_each(|table| transaction.execute_batch(table.ddl))?;
    stamp(transaction)
}

/// Setzt den Stempel auf [`SCHEMA_VERSION`] — **einmal**, am Ende von Anlage, Neubau oder Aufstieg.
fn stamp(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.pragma_update(None, "user_version", SCHEMA_VERSION)
}

/// Eine Sprosse der Leiter: hebt eine Datei von Version `from` auf `from + 1` — nie weiter, und ohne
/// zu stempeln (das tut `climb` einmal am Ende).
struct Rung {
    from: i64,
    migrate: fn(&Transaction<'_>) -> rusqlite::Result<()>,
}

/// Die Leiter, aufsteigend und lückenlos bis `SCHEMA_VERSION - 1` (ein Test wacht darüber). Eine neue
/// Migration hängt **eine** Sprosse ans Ende.
const LADDER: [Rung; 2] = [
    Rung {
        from: 2,
        migrate: add_category_register,
    },
    Rung {
        from: 3,
        migrate: add_outbox_status,
    },
];

/// Steigt von der vorgefundenen Version Sprosse für Sprosse bis [`SCHEMA_VERSION`] und stempelt dann.
fn climb(transaction: &Transaction<'_>, found: i64) -> rusqlite::Result<()> {
    LADDER
        .iter()
        .filter(|rung| rung.from >= found)
        .try_for_each(|rung| (rung.migrate)(transaction))?;
    stamp(transaction)
}

/// Sprosse 2 → 3 (Issue 0009): legt das Kategorie-Register samt seinem Index an. **Additiv** — keine
/// vorhandene Tabelle wird berührt, ihr Bestand bleibt, wie er ist.
fn add_category_register(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(LEARNED_CATEGORIES_TABLE.ddl)
}

/// Sprosse 3 → 4 (Issue 0085, ADR-0017): baut die Outbox auf die Form mit Status-Spalte und
/// Aussonderungs-Grund um und übernimmt jede Zeile — `processed_on_utc` gesetzt wird `processed`,
/// sonst `open`. Die `rowid` reist mit, weil sie die Zustell-Reihenfolge trägt.
///
/// **Umbau statt `ALTER TABLE ADD COLUMN`:** die Status-Spalte braucht die Tabellen-`CHECK`s, die
/// `ADD COLUMN` nicht nachrüsten kann, und erst die Neuanlage gibt der Datei **genau** das Schema einer
/// frischen. Die alte Tabelle wird dafür umbenannt, ihr Index vorher verworfen (sonst zöge er mit und
/// blockierte den Namen des neuen).
fn add_outbox_status(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(
        "DROP INDEX IF EXISTS outbox_open_idx; ALTER TABLE outbox RENAME TO outbox_version_3;",
    )?;
    transaction.execute_batch(OUTBOX_VERSION_4)?;
    transaction.execute_batch(
        "INSERT INTO outbox (rowid, id, message_type, partition_key, content, occurred_on_utc, \
                             attempt, status, processed_on_utc, created_at, updated_at) \
         SELECT rowid, id, message_type, partition_key, content, occurred_on_utc, attempt, \
                CASE WHEN processed_on_utc IS NULL THEN 'open' ELSE 'processed' END, \
                processed_on_utc, created_at, updated_at \
         FROM outbox_version_3; \
         DROP TABLE outbox_version_3;",
    )
}

/// Die Outbox, wie Sprosse 3 → 4 sie anlegt — **eingefroren**, nicht aus `OUTBOX_TABLE` gelesen: eine
/// spätere Änderung der Outbox darf diese Sprosse nicht verschieben, sondern bekommt ihre eigene. Dass
/// sie heute der aktuellen DDL gleicht, hält der Migrationstest fest (Schema-Vergleich mit einer
/// frischen Datei).
const OUTBOX_VERSION_4: &str = "\
CREATE TABLE outbox (
    id                 TEXT    PRIMARY KEY,
    message_type       TEXT    NOT NULL,
    partition_key      TEXT    NOT NULL,
    content            TEXT    NOT NULL DEFAULT '',
    occurred_on_utc    INTEGER NOT NULL,
    attempt            INTEGER NOT NULL DEFAULT 0,
    status             TEXT    NOT NULL DEFAULT 'open'
                               CHECK (status IN ('open', 'processed', 'dead_letter')),
    processed_on_utc   INTEGER,
    dead_letter_reason TEXT,
    created_at         INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at         INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    CHECK ((status = 'processed') = (processed_on_utc IS NOT NULL)),
    CHECK ((status = 'dead_letter') = (dead_letter_reason IS NOT NULL))
);
CREATE INDEX outbox_open_idx
    ON outbox (status) WHERE status = 'open';";

/// Version 1 (ADR-0016) — **keine Sprosse, sondern ein Neubau**, der direkt auf der **aktuellen**
/// Version landet: baut die beiden damals geänderten Tabellen (`einlesen_lines` +`row_key`,
/// `auswertungsfenster` +`run_id`) neu — **nur**, wenn beide leer sind. Leer heißt: nichts geht
/// verloren, und `DROP` + Neuanlage **aller** Tabellen aus der heutigen DDL gibt der Datei **genau**
/// das Schema, das eine frische Datei bekommt (kein `ALTER TABLE` mit Ersatz-Default, das davon
/// abwiche). Die Outbox wird wie in Sprosse 3 → 4 umgebaut, ihr Bestand bleibt. Die übrigen Tabellen
/// bleiben, ihr `CREATE … IF NOT EXISTS` läuft ins Leere; was seit Version 2 neu ist, entsteht dabei
/// gleich mit.
///
/// Trägt eine der beiden schon Zeilen, wird die Datei abgewiesen (siehe Modul-Doku) — innerhalb der
/// Transaktion von `ensure`, also unangetastet.
fn rebuild_unkeyed_v1(transaction: &Transaction<'_>) -> Result<(), SchemaError> {
    if has_rows(transaction, "einlesen_lines")? || has_rows(transaction, "auswertungsfenster")? {
        return Err(SchemaError::VersionMismatch(VersionMismatch {
            found: 1,
            expected: SCHEMA_VERSION,
            cause: MismatchCause::UnkeyedRows,
        }));
    }
    transaction.execute_batch("DROP TABLE einlesen_lines; DROP TABLE auswertungsfenster;")?;
    // Die Outbox steht noch in ihrer Form vor Version 4; `create` legte sie (`IF NOT EXISTS`) nicht
    // neu an. Ihr Umbau übernimmt den Bestand, wie es die Sprosse 3 → 4 tut.
    add_outbox_status(transaction)?;
    create(transaction)?;
    Ok(())
}

/// Trägt die Tabelle mindestens eine Zeile? Der Name ist stets ein Code-Literal, nie Fremd-Eingabe.
fn has_rows(conn: &Connection, table: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        &format!("SELECT EXISTS (SELECT 1 FROM {table});"),
        [],
        |row| row.get(0),
    )
}

/// **Alle** Tabellen des Programms: die der vier Ergebnis-Stores und die übrigen ([`OTHER_TABLES`]),
/// jede mit ihrer eigenen, beim Store authored DDL ([`TableSpec`]).
fn tables() -> impl Iterator<Item = &'static TableSpec> {
    store_tables().chain(OTHER_TABLES)
}

/// Die Tabellen der vier Ergebnis-Stores. Ein neuer Store, der hier fehlt, bekäme keine Tabelle — und
/// failte in der ersten Ablage terminal im Preflight. Die Liste muss darum **dieselben** Stores
/// aufführen, die `run` in seinen `Stores` verdrahtet.
fn store_tables() -> impl Iterator<Item = &'static TableSpec> {
    EinlesenLines::tables()
        .iter()
        .chain(ObfuskiertBookings::tables())
        .chain(FensternWindows::tables())
        .chain(AuswertenEvaluations::tables())
}

/// Die Tabellen, die zu **keinem** Ergebnis-Store gehören: die Outbox und das Kategorie-Register
/// (Issue 0009). Das Register steht **hinter** den Stores, weil seine DDL einen Index auf
/// `window_evaluation_fingerprints` legt.
const OTHER_TABLES: [&TableSpec; 2] = [&OUTBOX_TABLE, &LEARNED_CATEGORIES_TABLE];

/// Warum der Schema-Besitzer eine Datei nicht geöffnet hat. Jede Variante ist **redigiert**: sie
/// trägt Versionsnummern oder eine io-Diagnose der Zugriffs-Mechanik — **nie** einen Zeilenwert und
/// **nie** den Pfad der Datei (ein Pfad trägt die Verzeichnisstruktur der Maschine, womöglich einen
/// Kontonamen).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// Die Datei trägt eine andere Schema-Version als dieses Programm.
    VersionMismatch(VersionMismatch),
    /// Die Datei ließ sich nicht öffnen, lesen oder anlegen — die durchgereichte io-Diagnose.
    Unavailable(String),
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VersionMismatch(mismatch) => mismatch.fmt(f),
            Self::Unavailable(diagnosis) => write!(f, "Datenbank nicht verfügbar: {diagnosis}"),
        }
    }
}

impl std::error::Error for SchemaError {}

/// Redigiert **an der Quelle**: SQLite hängt an manche Meldungen den Dateipfad (etwa bei
/// `SQLITE_CANTOPEN`), und `rusqlite::Error::InvalidPath` trägt ihn ohnehin. Übernommen wird darum bei
/// einem SQLite-Fehler nur der **Fehlercode** samt seiner statischen Beschreibung, die erweiterte
/// Meldung wird verworfen; ein ungültiger Pfad wird zu einem statischen Text.
impl From<rusqlite::Error> for SchemaError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Unavailable(match error {
            rusqlite::Error::SqliteFailure(failure, _extended_message) => failure.to_string(),
            rusqlite::Error::InvalidPath(_path) => "ungültiger Datenbank-Pfad".to_string(),
            other => other.to_string(),
        })
    }
}

/// Die vorgefundene und die erwartete Schema-Version samt dem **Grund**, warum die Datei nicht
/// benutzt wird — zwei Zahlen und ein statischer Fall, sonst nichts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionMismatch {
    found: i64,
    expected: i64,
    cause: MismatchCause,
}

/// Warum eine Datei mit abweichender Version nicht benutzt wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchCause {
    /// Für diese Version kennt das Programm keinen Weg (eine neuere, eine ungestempelte mit Tabellen
    /// = Version 0, oder sonst eine fremde).
    UnknownVersion,
    /// Eine Version-1-Datei **mit** Bestand: ihre Zeilen tragen keinen Zeilen-Schlüssel und lassen
    /// sich nicht nachträglich schlüsseln (ADR-0016).
    UnkeyedRows,
}

impl VersionMismatch {
    /// Die Version, die die Datei trägt (`0` = ohne Stempel, aber mit Tabellen).
    #[must_use]
    pub const fn found(&self) -> i64 {
        self.found
    }

    /// Die Version, die dieses Programm erwartet ([`SCHEMA_VERSION`]).
    #[must_use]
    pub const fn expected(&self) -> i64 {
        self.expected
    }

    /// Warum die Datei nicht benutzt wird.
    #[must_use]
    pub const fn cause(&self) -> MismatchCause {
        self.cause
    }
}

impl std::fmt::Display for VersionMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cause {
            MismatchCause::UnknownVersion => write!(
                f,
                "die Datenbank trägt Schema-Version {}, dieses Programm erwartet {} — die Datei \
                 bleibt unangetastet; für diesen Lauf eine frische Datei angeben (Version 0 = \
                 angelegt vor dem Versionsstempel)",
                self.found, self.expected
            ),
            MismatchCause::UnkeyedRows => write!(
                f,
                "die Datenbank trägt Schema-Version {} mit eingelesenen Buchungen ohne \
                 Zeilen-Schlüssel; dieses Programm erwartet {} und kann sie nicht nachträglich \
                 schlüsseln — die Datei bleibt unangetastet; für diesen Lauf eine frische Datei \
                 angeben und die Auszüge neu einlesen",
                self.found, self.expected
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::einlesen::EinlesenSqliteStore;
    use crate::pipeline_test_support::TempDbPath;

    /// Liest über eine **eigene** Verbindung, was in der Datei steht — am Besitzer vorbei, damit die
    /// Prüfung nichts anlegt.
    fn roh(db: &TempDbPath) -> rusqlite::Result<Connection> {
        Connection::open(db.path())
    }

    fn user_version(conn: &Connection) -> rusqlite::Result<i64> {
        conn.pragma_query_value(None, "user_version", |row| row.get(0))
    }

    /// SQLites eigener Schema-Zähler: er wächst mit **jeder** Schema-Änderung. Bleibt er gleich, wurde
    /// nichts angelegt.
    fn schema_cookie(conn: &Connection) -> rusqlite::Result<i64> {
        conn.pragma_query_value(None, "schema_version", |row| row.get(0))
    }

    fn table_exists(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1;",
            [name],
            |row| row.get::<_, i64>(0).map(|count| count == 1),
        )
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_frische_datei_bekommt_alle_tabellen_und_die_schema_version()
    -> Result<(), Box<dyn std::error::Error>> {
        let db = TempDbPath::new("schema_frisch");

        drop(open(&AppConfig::new(db.path()))?);

        let conn = roh(&db)?;
        assert_eq!(user_version(&conn)?, SCHEMA_VERSION);
        // Jede Tabelle, die der Besitzer deklariert, steht in der Datei …
        for table in tables() {
            assert!(
                table_exists(&conn, table.name)?,
                "die Tabelle {} muss angelegt sein",
                table.name
            );
        }
        // … und gegen eine unabhängige Tatsache: SQLite zählt genau so viele Tabellen. Das belegt nur,
        // dass Deklaration und Datei übereinstimmen — dass `tables()` keinen Store vergisst, prüft der
        // Lauf (`run`), dessen Ablage sonst im Preflight scheiterte.
        let gezaehlt: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table';",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(usize::try_from(gezaehlt)?, tables().count());
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_passend_gestempelte_datei_wird_ohne_neuanlage_benutzt()
    -> Result<(), Box<dyn std::error::Error>> {
        let db = TempDbPath::new("schema_wieder");
        let config = AppConfig::new(db.path());
        drop(open(&config)?);
        let vorher = schema_cookie(&roh(&db)?)?;

        drop(open(&config)?);

        let conn = roh(&db)?;
        assert_eq!(
            schema_cookie(&conn)?,
            vorher,
            "das zweite Öffnen darf das Schema nicht anfassen"
        );
        assert_eq!(user_version(&conn)?, SCHEMA_VERSION);
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_datei_mit_fremder_version_wird_abgewiesen_und_bleibt_unangetastet()
    -> Result<(), Box<dyn std::error::Error>> {
        let db = TempDbPath::new("schema_fremd");
        let fremd = SCHEMA_VERSION + 1;
        roh(&db)?.pragma_update(None, "user_version", fremd)?;

        let fehler = open(&AppConfig::new(db.path())).err();

        assert_eq!(
            fehler,
            Some(SchemaError::VersionMismatch(VersionMismatch {
                found: fremd,
                expected: SCHEMA_VERSION,
                cause: MismatchCause::UnknownVersion,
            }))
        );
        let conn = roh(&db)?;
        assert_eq!(
            user_version(&conn)?,
            fremd,
            "der Stempel bleibt, wie er war"
        );
        assert!(
            !table_exists(&conn, OUTBOX_TABLE.name)?,
            "eine abgewiesene Datei bekommt keine Tabelle"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_ungestempelte_datei_mit_tabellen_gilt_als_version_null_und_wird_abgewiesen()
    -> Result<(), Box<dyn std::error::Error>> {
        // Eine Datei aus der Zeit vor dem Stempel: Tabellen ja, `user_version` 0.
        let db = TempDbPath::new("schema_alt");
        roh(&db)?.execute_batch(OUTBOX_TABLE.ddl)?;
        let vorher = schema_cookie(&roh(&db)?)?;

        let fehler = open(&AppConfig::new(db.path())).err();

        assert_eq!(
            fehler,
            Some(SchemaError::VersionMismatch(VersionMismatch {
                found: 0,
                expected: SCHEMA_VERSION,
                cause: MismatchCause::UnknownVersion,
            }))
        );
        let conn = roh(&db)?;
        assert_eq!(
            user_version(&conn)?,
            0,
            "die Alt-Datei wird nicht gestempelt"
        );
        assert_eq!(
            schema_cookie(&conn)?,
            vorher,
            "und nicht um Tabellen ergänzt"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ein_store_legt_kein_eigenes_schema_an() -> rusqlite::Result<()> {
        // Der produktive Store-Konstruktor über einer völlig leeren Datenbank: er darf nichts anlegen
        // — Schema bekommt eine Datenbank allein über den Besitzer.
        let outbox = SqliteOutbox::from_connection(Connection::open_in_memory()?);

        let _store = EinlesenSqliteStore::new(outbox.clone());

        let objekte: i64 = outbox.with_connection(|conn| {
            conn.query_row("SELECT count(*) FROM sqlite_master;", [], |row| row.get(0))
        })?;
        assert_eq!(objekte, 0, "der Store hat ein Schema-Objekt angelegt");
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn jede_tabelle_traegt_die_audit_spalten() -> Result<(), Box<dyn std::error::Error>> {
        // Das Netz für `.rules/sql/schema-konventionen.md`, seit `ensure_columns` mit Issue 0084
        // entfallen ist: jede deklarierte Tabelle hat `created_at` und `updated_at` in ihrer DDL.
        let outbox = open_in_memory()?;
        for table in tables() {
            let spalten: Vec<String> = outbox.with_connection(|conn| {
                let mut statement = conn.prepare(&format!("PRAGMA table_info({});", table.name))?;
                let namen = statement.query_map([], |row| row.get(1))?;
                namen.collect()
            })?;
            for audit in ["created_at", "updated_at"] {
                assert!(
                    spalten.iter().any(|spalte| spalte == audit),
                    "der Tabelle {} fehlt {audit}: {spalten:?}",
                    table.name
                );
            }
        }
        Ok(())
    }

    #[test]
    fn eine_nicht_oeffenbare_datei_nennt_ihren_pfad_nicht() {
        // Ein Verzeichnisname, der wie ein Kontoname aussieht — er darf in keiner Meldung landen.
        let pfad = std::env::temp_dir()
            .join(format!("Konto_Mustermann_{}", std::process::id()))
            .join("lauf.sqlite");

        let fehler = open(&AppConfig::new(&pfad)).err();

        assert!(
            matches!(fehler, Some(SchemaError::Unavailable(_))),
            "eine Datei in einem fehlenden Verzeichnis ist nicht verfügbar: {fehler:?}"
        );
        let meldung = fehler.map(|fehler| fehler.to_string()).unwrap_or_default();
        assert!(!meldung.contains("Konto_Mustermann"), "{meldung}");
        assert!(!meldung.contains(&*pfad.to_string_lossy()), "{meldung}");
    }

    #[test]
    fn die_meldung_bei_falscher_version_nennt_nur_die_versionen() {
        let meldung = SchemaError::VersionMismatch(VersionMismatch {
            found: 0,
            expected: SCHEMA_VERSION,
            cause: MismatchCause::UnknownVersion,
        })
        .to_string();

        assert!(meldung.contains("Schema-Version 0"), "{meldung}");
        assert!(
            meldung.contains(&format!("erwartet {SCHEMA_VERSION}")),
            "{meldung}"
        );
    }

    // --- Migration 1 → 2 (ADR-0016) -----------------------------------------------------------

    /// Die beiden Tabellen in ihrer **Version-1-Form** — eingefroren als Test-Wahrheit, weil die
    /// Stores nur noch die heutige DDL kennen: `einlesen_lines` ohne `row_key`,
    /// `auswertungsfenster` ohne `run_id` (und ohne dessen Index).
    const V1_EINLESEN_LINES: &str = "\
CREATE TABLE einlesen_lines (
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

    const V1_AUSWERTUNGSFENSTER: &str = "\
CREATE TABLE auswertungsfenster (
    id         TEXT    PRIMARY KEY,
    from_day   INTEGER NOT NULL,
    to_day     INTEGER NOT NULL,
    currency   TEXT    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
);";

    /// Eine erfundene, gültige Auftrags-IBAN — sie darf in keiner Meldung auftauchen.
    const ALT_IBAN: &str = "DE93100000000000000100";

    /// Legt eine Datei an, wie Version 1 sie hinterließ: alle Tabellen jener Zeit, die beiden
    /// geänderten in ihrer alten Form, **ohne** das Register (es kam erst mit Version 3), die Outbox
    /// ohne Status-Spalte (die kam mit Version 4), Stempel 1.
    fn version_1_datei(name: &str) -> Result<TempDbPath, Box<dyn std::error::Error>> {
        let db = TempDbPath::new(name);
        drop(open(&AppConfig::new(db.path()))?);
        let conn = roh(&db)?;
        conn.execute_batch(&format!(
            "DROP TABLE einlesen_lines; DROP TABLE auswertungsfenster; \
             {OHNE_REGISTER} {V1_EINLESEN_LINES} {V1_AUSWERTUNGSFENSTER} {ALTE_OUTBOX}"
        ))?;
        conn.pragma_update(None, "user_version", 1)?;
        Ok(db)
    }

    /// Das ganze Schema einer Datei als vergleichbarer Text — jede Tabelle und jeder Index mit
    /// seiner DDL, nach Namen geordnet.
    fn schema_text(conn: &Connection) -> rusqlite::Result<Vec<(String, Option<String>)>> {
        let mut statement = conn.prepare("SELECT name, sql FROM sqlite_master ORDER BY name;")?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect()
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_version_1_datei_ohne_bestand_wird_auf_die_aktuelle_version_migriert()
    -> Result<(), Box<dyn std::error::Error>> {
        let alt = version_1_datei("schema_v1_leer")?;
        let frisch = TempDbPath::new("schema_v2_frisch");
        drop(open(&AppConfig::new(frisch.path()))?);

        drop(open(&AppConfig::new(alt.path()))?);

        let conn = roh(&alt)?;
        assert_eq!(user_version(&conn)?, SCHEMA_VERSION);
        // Danach trägt die Datei **genau** das Schema einer frisch angelegten — Spalte für Spalte,
        // Index für Index. Ein Umbau, der davon abwiche, wäre die nächste Migrations-Falle.
        assert_eq!(schema_text(&conn)?, schema_text(&roh(&frisch)?)?);
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_version_1_datei_mit_bestand_wird_terminal_redigiert_abgewiesen_und_bleibt_unangetastet()
    -> Result<(), Box<dyn std::error::Error>> {
        let alt = version_1_datei("schema_v1_bestand")?;
        roh(&alt)?.execute(
            "INSERT INTO einlesen_lines (id, ordering_account, amount_minor_units, currency, \
             direction, booking_year, booking_month, booking_day, value_year, value_month, \
             value_day) VALUES ('alt-1', ?1, 490, 'EUR', 'outgoing', 2026, 6, 30, 2026, 6, 30);",
            [ALT_IBAN],
        )?;
        let vorher = schema_cookie(&roh(&alt)?)?;

        let fehler = open(&AppConfig::new(alt.path())).err();

        assert_eq!(
            fehler,
            Some(SchemaError::VersionMismatch(VersionMismatch {
                found: 1,
                expected: SCHEMA_VERSION,
                cause: MismatchCause::UnkeyedRows,
            }))
        );
        let meldung = fehler.map(|fehler| fehler.to_string()).unwrap_or_default();
        assert!(meldung.contains("Zeilen-Schlüssel"), "{meldung}");
        assert!(!meldung.contains(ALT_IBAN), "{meldung}");
        // Unangetastet: Stempel, Schema und Bestand wie vorher.
        let conn = roh(&alt)?;
        assert_eq!(user_version(&conn)?, 1);
        assert_eq!(schema_cookie(&conn)?, vorher);
        let zeilen: i64 =
            conn.query_row("SELECT count(*) FROM einlesen_lines;", [], |row| row.get(0))?;
        assert_eq!(zeilen, 1);
        Ok(())
    }

    // --- Die Leiter und Sprosse 2 → 3 (Issue 0009) -------------------------------------------

    /// Nimmt einer frischen Datei, was Version 3 hinzufügte: das Register und seinen Index.
    const OHNE_REGISTER: &str = "DROP TABLE learned_categories; \
         DROP INDEX window_evaluation_fingerprints_fingerprint_idx;";

    #[test]
    fn die_leiter_reicht_lueckenlos_bis_zur_aktuellen_version() {
        // Jede Sprosse hebt um genau eine Version; zusammen schließen sie an Version 2 an (Version 1
        // ist ein Neubau neben der Leiter) und enden direkt unter der aktuellen.
        let von: Vec<i64> = LADDER.iter().map(|rung| rung.from).collect();
        assert_eq!(von, (2..SCHEMA_VERSION).collect::<Vec<_>>());
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_version_2_datei_mit_bestand_bekommt_das_register_und_behaelt_ihren_bestand()
    -> Result<(), Box<dyn std::error::Error>> {
        // Eine Datei, wie Version 2 sie hinterließ: alles außer dem Register — und mit Bestand.
        let alt = TempDbPath::new("schema_v2_bestand");
        drop(open(&AppConfig::new(alt.path()))?);
        let conn = roh(&alt)?;
        conn.execute_batch(&format!("{OHNE_REGISTER} {ALTE_OUTBOX}"))?;
        conn.pragma_update(None, "user_version", 2)?;
        conn.execute(
            "INSERT INTO outbox (id, message_type, partition_key, occurred_on_utc) \
             VALUES ('alt-1', 'ReadTransactions', 'auszug.csv', 0);",
            [],
        )?;
        drop(conn);
        let frisch = TempDbPath::new("schema_v3_frisch");
        drop(open(&AppConfig::new(frisch.path()))?);

        drop(open(&AppConfig::new(alt.path()))?);

        let conn = roh(&alt)?;
        assert_eq!(user_version(&conn)?, SCHEMA_VERSION);
        assert!(table_exists(&conn, LEARNED_CATEGORIES_TABLE.name)?);
        // Genau das Schema einer frischen Datei — und der Bestand ist noch da.
        assert_eq!(schema_text(&conn)?, schema_text(&roh(&frisch)?)?);
        let nachrichten: i64 =
            conn.query_row("SELECT count(*) FROM outbox;", [], |row| row.get(0))?;
        assert_eq!(nachrichten, 1);
        Ok(())
    }

    // --- Sprosse 3 → 4 (Issue 0085) -----------------------------------------------------------

    /// Ersetzt die Outbox einer frischen Datei durch ihre Form **vor** Version 4: ohne Status-Spalte,
    /// offen hieß `processed_on_utc IS NULL`. Eingefroren wie die Version-1-Tabellen oben.
    const ALTE_OUTBOX: &str = "\
DROP TABLE outbox;
CREATE TABLE outbox (
    id              TEXT    PRIMARY KEY,
    message_type    TEXT    NOT NULL,
    partition_key   TEXT    NOT NULL,
    content         TEXT    NOT NULL DEFAULT '',
    occurred_on_utc INTEGER NOT NULL,
    attempt         INTEGER NOT NULL DEFAULT 0,
    processed_on_utc INTEGER,
    created_at      INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at      INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
);
CREATE INDEX outbox_open_idx
    ON outbox (processed_on_utc) WHERE processed_on_utc IS NULL;";

    /// `rowid`, ID, Status, Versuche und Ack-Stempel jeder Outbox-Zeile in Zustell-Reihenfolge.
    type Nachricht = (i64, String, String, i64, Option<i64>);

    fn nachrichten(conn: &Connection) -> rusqlite::Result<Vec<Nachricht>> {
        let mut statement = conn.prepare(
            "SELECT rowid, id, status, attempt, processed_on_utc FROM outbox ORDER BY rowid;",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
        rows.collect()
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_version_3_datei_mit_bestand_bekommt_die_status_spalte_und_behaelt_jede_nachricht()
    -> Result<(), Box<dyn std::error::Error>> {
        // Eine Outbox mit Geschichte und **nicht aufsteigend** eingefügten `rowid`s: zuerst eine
        // verarbeitete Nachricht auf `rowid` 7, dann eine offene mit einem gescheiterten Versuch auf
        // `rowid` 3. Die Zustell-Reihenfolge hängt an der `rowid` — die Migration muss sie mitnehmen.
        let alt = TempDbPath::new("schema_v3_bestand");
        drop(open(&AppConfig::new(alt.path()))?);
        let conn = roh(&alt)?;
        conn.execute_batch(ALTE_OUTBOX)?;
        conn.pragma_update(None, "user_version", 3)?;
        conn.execute_batch(
            "INSERT INTO outbox (rowid, id, message_type, partition_key, occurred_on_utc, \
                                 attempt, processed_on_utc) VALUES (7, 'erst', 'A', 'k1', 1, 0, 5); \
             INSERT INTO outbox (rowid, id, message_type, partition_key, occurred_on_utc, \
                                 attempt) VALUES (3, 'dann', 'B', 'k2', 2, 1);",
        )?;
        drop(conn);
        let frisch = TempDbPath::new("schema_v4_frisch");
        drop(open(&AppConfig::new(frisch.path()))?);

        drop(open(&AppConfig::new(alt.path()))?);

        let conn = roh(&alt)?;
        assert_eq!(user_version(&conn)?, SCHEMA_VERSION);
        assert_eq!(SCHEMA_VERSION, 4);
        assert_eq!(
            nachrichten(&conn)?,
            [
                (3, "dann".to_string(), "open".to_string(), 1, None),
                (7, "erst".to_string(), "processed".to_string(), 0, Some(5)),
            ],
            "jede Nachricht bleibt, auf ihrer rowid, mit ihrem Stand"
        );
        // Genau das Schema einer frischen Datei — die eingefrorene DDL der Sprosse gleicht der heutigen.
        assert_eq!(schema_text(&conn)?, schema_text(&roh(&frisch)?)?);
        Ok(())
    }
}
