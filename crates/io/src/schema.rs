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
//! **Wo eine Migration ansetzt.** Die Versionen laufen über die eine Weiche `ensure`: heute kennt es nur „frisch"
//! und „passt". Eine Migration von Version *n* auf *n + 1* ist dort **ein** weiterer Arm
//! (`Found::Version(1) => migrate_1_to_2(conn)`), dazu [`SCHEMA_VERSION`] hochgezählt — und die
//! DDL der Stores beschreibt dann das **neue** Schema, das eine frische Datei direkt bekommt. Eine
//! Migrations-Maschinerie auf Vorrat gibt es bewusst nicht.

use rusqlite::Connection;

use crate::auswerten::AuswertenEvaluations;
use crate::config::AppConfig;
use crate::einlesen::EinlesenLines;
use crate::fenstern::FensternWindows;
use crate::obfuskierung::ObfuskiertBookings;
use crate::sqlite::{OUTBOX_TABLE, SqliteOutbox, StoreSpec, TableSpec};

/// Die Schema-Version **dieses Programms** — der Wert, mit dem eine frisch angelegte Datei gestempelt
/// wird und den eine vorgefundene Datei tragen muss. Hochzählen nur zusammen mit einer Migration in
/// `ensure`.
pub const SCHEMA_VERSION: i64 = 1;

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
fn ensure(conn: &Connection) -> Result<(), SchemaError> {
    match found(conn)? {
        Found::Empty => create(conn),
        Found::Version(SCHEMA_VERSION) => Ok(()),
        Found::Version(found) => Err(SchemaError::VersionMismatch(VersionMismatch {
            found,
            expected: SCHEMA_VERSION,
        })),
    }
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

/// Legt **alle** Tabellen an und stempelt die Datei — in einer Transaktion: scheitert ein Schritt,
/// bleibt die Datei leer und ungestempelt, also beim nächsten Öffnen wieder „frisch".
fn create(conn: &Connection) -> Result<(), SchemaError> {
    let transaction = conn.unchecked_transaction()?;
    tables().try_for_each(|table| transaction.execute_batch(table.ddl))?;
    transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    transaction.commit()?;
    Ok(())
}

/// **Alle** Tabellen des Programms: die Outbox und die der vier Ergebnis-Stores, jede mit ihrer
/// eigenen, beim Slice-Store authored DDL ([`TableSpec`]). Ein neuer Store, der hier fehlt, bekäme
/// keine Tabelle — und failte in der ersten Ablage terminal im Preflight.
fn tables() -> impl Iterator<Item = &'static TableSpec> {
    std::iter::once(&OUTBOX_TABLE)
        .chain(EinlesenLines::tables())
        .chain(ObfuskiertBookings::tables())
        .chain(FensternWindows::tables())
        .chain(AuswertenEvaluations::tables())
}

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

impl From<rusqlite::Error> for SchemaError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Unavailable(error.to_string())
    }
}

/// Die vorgefundene und die erwartete Schema-Version — zwei Zahlen, sonst nichts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionMismatch {
    found: i64,
    expected: i64,
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
}

impl std::fmt::Display for VersionMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "die Datenbank trägt Schema-Version {}, dieses Programm erwartet {} — die Datei bleibt \
             unangetastet; für diesen Lauf eine frische Datei angeben (Version 0 = angelegt vor dem \
             Versionsstempel)",
            self.found, self.expected
        )
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
        // Die Outbox und **jede** Tabelle aller vier Stores — über die Liste, die der Besitzer selbst
        // benutzt, und gegen die Listen der Stores direkt.
        let erwartet = std::iter::once(&OUTBOX_TABLE)
            .chain(EinlesenLines::tables())
            .chain(ObfuskiertBookings::tables())
            .chain(FensternWindows::tables())
            .chain(AuswertenEvaluations::tables());
        for table in erwartet {
            assert!(
                table_exists(&conn, table.name)?,
                "die Tabelle {} muss angelegt sein",
                table.name
            );
        }
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

    #[test]
    fn die_meldung_bei_falscher_version_nennt_nur_die_versionen() {
        let meldung = SchemaError::VersionMismatch(VersionMismatch {
            found: 0,
            expected: SCHEMA_VERSION,
        })
        .to_string();

        assert!(meldung.contains("Schema-Version 0"), "{meldung}");
        assert!(
            meldung.contains(&format!("erwartet {SCHEMA_VERSION}")),
            "{meldung}"
        );
    }
}
