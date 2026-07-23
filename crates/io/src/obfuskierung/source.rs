//! Die reale io-Erfüllung des öffentlichen [`IngestedSourceSeam`] des `obfuskierung`-Adapters
//! ([Issue 0056](../../../../docs/issues/0056-obfuskierung-store-real-und-end-to-end.md)): der
//! **Lese-Seam über die Datenschutz-Grenze**, der die rohe `einlesen`-Zeile **per ID** aus der realen
//! `einlesen_lines`-Tabelle liest ([Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md)).
//! Er tritt an die Stelle des `pub(crate)`-Fakes der Specs — das direkte Gegenstück zu `einlesen`s
//! `CsvCamtV8Source`.
//!
//! **Außerhalb der committenden Transaktion (ADR-0012, uniformes ID-Handoff):** die `einlesen`-Zeile
//! ist **append-only/unveränderlich**, also ist ihr Lesen message-unabhängig und idempotent — es
//! läuft **vor** der Arbeitseinheit (`SqliteOutbox::with_connection`, eine reine Lese-Leihgabe, die
//! endet, bevor der spätere Commit die Verbindung schreibend leiht; strikt serielle Verarbeitung,
//! siehe [`SqliteOutbox`](crate::sqlite::SqliteOutbox)).
//!
//! **Fehlbar → terminal, redigiert:** der Seam liefert `Option<IngestedRow>` — eine fehlende **oder**
//! unlesbare Zeile wird zu `None`, das die adapter-seitige `SourceBridge` in den terminalen,
//! redigierten `SourceError::NotFound` (nur die opake ID, nie ein Rohwert) mappt. Kein Dead-Letter,
//! kein Retry.
//!
//! **Format-Rekonstruktion:** die `einlesen`-Zeile legt Datum als drei Integer-Spalten (Jahr/Monat/Tag)
//! ab; der Seam setzt sie zum kanonischen `YYYY-MM-DD`-Textstamp zusammen (die Pipeline-Konvention, die
//! `obfuskierung` als Pass-Through verbatim weiterreicht). `direction` reist als der stabile
//! `einlesen`-Speicher-Token (`incoming`/`outgoing`) durch — der `obfuskierung`-Slice deutet ihn nicht,
//! er trägt ihn nur.

use obfuskierung_adapter::{IngestedRow, IngestedSourceSeam};
use rusqlite::OptionalExtension;

use crate::sqlite::SqliteOutbox;

/// Der reale einlesen-Lese-Seam: liest die rohe `einlesen_lines`-Zeile zur ID über das geteilte
/// SQLite-Rückgrat. `Clone` teilt dasselbe Rückgrat (`Rc` im Innern von [`SqliteOutbox`]).
#[derive(Clone)]
pub struct EinlesenRowSource {
    outbox: SqliteOutbox,
}

impl EinlesenRowSource {
    /// Bindet den Seam an das gegebene SQLite-Rückgrat (auf dem auch der `einlesen`-Store schreibt).
    /// Legt **nichts** an — die `einlesen_lines`-Tabelle gehört dem `einlesen`-Store; dieser Seam
    /// liest nur.
    pub fn new(outbox: SqliteOutbox) -> Self {
        Self { outbox }
    }
}

impl IngestedSourceSeam for EinlesenRowSource {
    /// Liest die rohe `einlesen`-Zeile zur ID. `None` bedeutet **fehlt oder unlesbar** (eine
    /// fehlende Zeile wie auch ein Lese-/Schema-Fehler) — beides mappt die `SourceBridge` auf den
    /// terminalen, redigierten `SourceError::NotFound`. Der Seam bietet keinen eigenen Fehlerkanal,
    /// darum die bewusste Zusammenfassung „nicht verwertbar → `None`".
    fn read(&self, id: &str) -> Option<IngestedRow> {
        self.outbox
            .with_connection(|conn| {
                conn.query_row(SELECT_LINE, [id], |row| {
                    Ok(IngestedRow {
                        ordering_account: row.get(0)?,
                        counter_account: row.get(1)?,
                        amount_minor_units: u64::try_from(row.get::<_, i64>(2)?)
                            .map_err(|_negative| negative_amount(2))?,
                        currency: row.get(3)?,
                        direction: row.get(4)?,
                        recipient_name: row.get(5)?,
                        creditor_id: row.get(6)?,
                        booking_day: iso_day(row.get(7)?, row.get(8)?, row.get(9)?),
                        value_date: iso_day(row.get(10)?, row.get(11)?, row.get(12)?),
                        purpose: row.get(13)?,
                        booking_text: row.get(14)?,
                    })
                })
                .optional()
            })
            // Ein `rusqlite`-Fehler (fehlendes Schema o. Ä.) wird — wie ein fehlender Treffer — zu
            // `None`; der Seam-Vertrag kennt keinen Fehlerkanal, und stromabwärts ist beides terminal.
            .ok()
            .flatten()
    }
}

/// Setzt die drei Datums-Integer-Spalten zum kanonischen `YYYY-MM-DD`-Textstamp zusammen
/// (null-gepolstert) — die Pipeline-Konvention, die `obfuskierung` als Pass-Through weiterträgt.
fn iso_day(year: i64, month: i64, day: i64) -> String {
    format!("{year:04}-{month:02}-{day:02}")
}

/// Ein negativer Betrag im Bestand (kann nur bei manuell verfälschter DB feuern) — als
/// `rusqlite`-Konvertierungsfehler; führt über `read`s `.ok()` zu `None` (terminal stromabwärts).
/// Der Marker ist ein interner Text, nie ein Roh-Zellwert.
fn negative_amount(column: usize) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        rusqlite::types::Type::Integer,
        "unexpected negative stored amount".into(),
    )
}

/// Selektiert genau die vom [`IngestedRow`] verwerteten Spalten der `einlesen_lines`-Zeile zur ID
/// (nie `SELECT *`, `.rules/sql/queries-und-performance.md`). Die Datums-Achsen kommen als
/// Jahr/Monat/Tag-Tripel, die [`iso_day`] zum Textstamp fügt.
const SELECT_LINE: &str = "\
SELECT ordering_account, counter_account, amount_minor_units, currency, direction,
       recipient_name, creditor_id,
       booking_year, booking_month, booking_day,
       value_year, value_month, value_day,
       purpose, booking_text
FROM einlesen_lines
WHERE id = ?1;";
