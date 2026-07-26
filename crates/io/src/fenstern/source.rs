//! Die reale io-Erfüllung des öffentlichen [`ObfuscatedWindowSeam`] des `fenstern`-Adapters
//! ([Issue 0057](../../../../docs/issues/0057-fenstern-store-real-und-end-to-end.md)): der
//! **Lese-Seam über die reale `obfuskiert`-Tabelle**
//! ([Issue 0056](../../../../docs/issues/0056-obfuskierung-store-real-und-end-to-end.md)). Er tritt
//! an die Stelle des `FakeSource` der Fixtures — das direkte Gegenstück zu `obfuskierung`s
//! `EinlesenRowSource`.
//!
//! Ein Seam, drei Fragen: die **Seiten-Lese** (`[from … to]`) und die **beiden Anker-Fragen**
//! (`latest`/`latest_before`) laufen alle über dieselbe `obfuskiert`-Tabelle — der Adapter deckt sie
//! mit **einem** Trait ([`ObfuscatedWindowSeam`]), also erfüllt sie hier **ein** io-Struct.
//!
//! **Außerhalb der committenden Transaktion** (ADR-0012, uniformes ID-Handoff): die `obfuskiert`-Zeile
//! ist append-only, ihr Lesen also message-unabhängig und idempotent — es läuft über
//! [`SqliteOutbox::with_connection`], eine reine Lese-Leihgabe.
//!
//! **Tag-Stamp ↔ ISO-Text:** die Domäne rechnet auf einem sortierbaren `i64`-Tag-Stamp (Tage seit
//! der Unix-Epoche — die Achse, die `auswerten`s Kalender-Rekonstruktion definiert und die `fenstern`
//! **teilt**), die `obfuskiert`-Tabelle trägt `booking_day` als `YYYY-MM-DD`-Text. Die Übersetzung
//! zwischen beiden Darstellungen besorgt der **geteilte** [`calendar`](crate::common::calendar)-Helfer,
//! den auch `auswerten` benutzt — eine Quelle je Achse. Die SQL sortiert/vergleicht ohnehin auf der
//! `TEXT`-Spalte (`MAX(booking_day)`/`booking_day < ?`), unabhängig von der `i64`-Epoche.
//!
//! **Fehlbar → terminal:** ein `Err` ist stromabwärts terminal
//! (`WindowError::SourceUnavailable { reason }`, [`AnchorBridge`](fenstern_adapter) reicht die
//! io-Diagnose unverändert durch — anders als bei `einlesen`/`obfuskierung` trägt diese Meldung
//! **keine** personenbezogenen Rohwerte, weil dieser Seam ausschließlich stromabwärts der
//! Datenschutz-Grenze liest: nur opake IDs, eine Währung und Tag-Stamps).

use fenstern_adapter::{ObfuscatedRow, ObfuscatedWindowSeam, SourceUnavailable};
use rusqlite::params;

use crate::common::calendar;
use crate::sqlite::SqliteOutbox;

/// Der reale Lese-Seam über die `obfuskiert`-Tabelle. `Clone` teilt dasselbe Rückgrat (`Rc` im
/// Innern von [`SqliteOutbox`]).
#[derive(Clone)]
pub struct ObfuskiertWindowSource {
    outbox: SqliteOutbox,
}

impl ObfuskiertWindowSource {
    /// Bindet den Seam an das gegebene SQLite-Rückgrat (auf dem auch der `obfuskierung`-Store
    /// schreibt). Legt **nichts** an — die `obfuskiert`-Tabelle gehört dem `obfuskierung`-Store;
    /// dieser Seam liest nur.
    pub fn new(outbox: SqliteOutbox) -> Self {
        Self { outbox }
    }
}

impl ObfuscatedWindowSeam for ObfuskiertWindowSource {
    /// Liest die Seite `[from … to]` (beide Grenzen einschließlich), neu→alt.
    fn read(&self, from: i64, to: i64) -> Result<Vec<ObfuscatedRow>, SourceUnavailable> {
        let from_day = iso_day(from)?;
        let to_day = iso_day(to)?;
        self.outbox
            .with_connection(|conn| {
                let mut statement = conn.prepare(SELECT_PAGE)?;
                let rows = statement.query_map(params![from_day, to_day], |row| {
                    Ok(ObfuscatedRow {
                        id: row.get(0)?,
                        currency: row.get(1)?,
                    })
                })?;
                rows.collect()
            })
            .map_err(unavailable)
    }

    /// Der jüngste Buchungstag des ganzen Bestands — der Initial-Anker.
    fn latest(&self) -> Result<Option<i64>, SourceUnavailable> {
        self.outbox
            .with_connection(|conn| conn.query_row(SELECT_LATEST, [], |row| row.get(0)))
            .map_err(unavailable)
            .and_then(|found: Option<String>| found.map(day_stamp).transpose())
    }

    /// Der jüngste Buchungstag echt unterhalb von `day`.
    fn latest_before(&self, day: i64) -> Result<Option<i64>, SourceUnavailable> {
        let before = iso_day(day)?;
        self.outbox
            .with_connection(|conn| {
                conn.query_row(SELECT_LATEST_BEFORE, params![before], |row| row.get(0))
            })
            .map_err(unavailable)
            .and_then(|found: Option<String>| found.map(day_stamp).transpose())
    }
}

/// Setzt den `i64`-Tag-Stamp zum kanonischen `YYYY-MM-DD`-Textstamp um (das Gegenstück zu
/// [`day_stamp`]) — `Err`, wenn der Stamp jenseits des von `chrono` darstellbaren Kalenderbereichs
/// liegt (praktisch unerreichbar: der Stamp stammt stets aus einem zuvor per [`day_stamp`] gelesenen
/// echten Kalendertag).
fn iso_day(day: i64) -> Result<String, SourceUnavailable> {
    calendar::iso_day(day).ok_or_else(|| {
        unavailable_message("Tag-Stamp außerhalb des darstellbaren Kalenderbereichs")
    })
}

/// Deutet einen gespeicherten `YYYY-MM-DD`-Textstamp zum sortierbaren `i64`-Tag-Stamp (das
/// Gegenstück zu [`iso_day`]) — `Err`, wenn der Bestand einen unlesbaren Textstamp trägt.
fn day_stamp(iso: String) -> Result<i64, SourceUnavailable> {
    calendar::day_stamp(&iso)
        .ok_or_else(|| unavailable_message("gespeicherter Buchungstag unlesbar"))
}

fn unavailable_message(message: &str) -> SourceUnavailable {
    SourceUnavailable {
        message: message.to_string(),
    }
}

fn unavailable(err: rusqlite::Error) -> SourceUnavailable {
    SourceUnavailable {
        message: format!("obfuskiert-Bestand nicht lesbar: {err}"),
    }
}

/// Selektiert genau die vom [`ObfuscatedRow`] verwerteten Spalten der Seite `[from … to]`, neu→alt
/// (nie `SELECT *`, `.rules/sql/queries-und-performance.md`). `rowid` bricht Gleichstand
/// deterministisch (Einreih-Reihenfolge innerhalb desselben Buchungstags).
const SELECT_PAGE: &str = "\
SELECT id, currency FROM obfuskiert
WHERE booking_day BETWEEN ?1 AND ?2
ORDER BY booking_day DESC, rowid ASC;";

const SELECT_LATEST: &str = "SELECT MAX(booking_day) FROM obfuskiert;";

const SELECT_LATEST_BEFORE: &str =
    "SELECT MAX(booking_day) FROM obfuskiert WHERE booking_day < ?1;";
