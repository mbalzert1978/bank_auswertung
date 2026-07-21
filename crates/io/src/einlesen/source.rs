//! Die echte io-Erfüllung des öffentlichen [`SourceSeam`] des `einlesen`-Adapters
//! ([Issue 0051](../../../../docs/issues/0051-einlesen-csv-camt-v8-quelle-real-hinter-source-seam.md)):
//! ein **Adapter**, der eine CSV-CAMT-v8-Datei über `fs` liest, ihre Zeilen mit der battle-tested
//! [`csv`]-Lib zu [`RawRecord`]s parst und sie **in-process** über den Seam an den `einlesen`-Slice
//! reicht (der Handler *fetcht* den Batch über den Port,
//! [Issue 0018](../../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)).
//! Sie tritt an die Stelle des früheren In-Memory-Quell-Fakes — **gleiche Signatur**.
//!
//! **Spaltenlayout:** Standard-Sparkassen-Header (`docs/examples/csv_camt_v8_header.md`),
//! `;`-getrennt und `"`-gequotet; die verwerteten Spalten holt der Adapter **positionsbasiert** (die
//! übrigen sind für den Slice belanglos). Die **rohe IBAN reist** über den `RawRecord` weiter (sie
//! darf lokal existieren; obfuskiert wird der Folge-Slice), die **BIC** liest der Adapter gar nicht
//! erst ein — sie bleibt io-lokal, kein Egress
//! ([ADR-0008](../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)).
//!
//! **Reines Lesen/Parsen — keine fachliche Zusicherung hier.** Der Reader liefert nur die rohen
//! `RawRecord`s. Die **formatunabhängigen** Inbound-Zusicherungen (eine Währung über den ganzen
//! Batch **und** die Präsenz der Pflichtfelder) liegen bewusst **nicht** im CSV-Reader, sondern in
//! der Adapter-ACL (`SourceBridge` hinter dem Seam) — so erbt sie jedes spätere Quellformat
//! (MT940/XML). Eine leere Zelle wird zu `None` (der Domänen-`NonEmptyString` trimmt ohnehin). Nur
//! ein **Lese-/Parse-Fehler** ist hier terminal: er trägt die durchgereichte io-Diagnose
//! (Pfad/OS/Parse-Mechanik, nie Zellinhalt).

use std::path::{Path, PathBuf};

use csv::{ReaderBuilder, StringRecord};
use einlesen_adapter::{RawRecord, SourceSeam, SourceUnavailable};

// Positionen der verwerteten Spalten im Standard-Sparkassen-Header (0-indexiert). Die BIC (Spalte 13)
// fehlt bewusst — sie wird nie eingelesen und bleibt io-lokal.
const COL_ORDERING_ACCOUNT: usize = 0;
const COL_BOOKING_DAY: usize = 1;
const COL_VALUE_DATE: usize = 2;
const COL_BOOKING_TEXT: usize = 3;
const COL_PURPOSE: usize = 4;
const COL_CREDITOR_ID: usize = 5;
const COL_RECIPIENT_NAME: usize = 11;
const COL_COUNTER_ACCOUNT: usize = 12;
const COL_AMOUNT: usize = 14;
const COL_CURRENCY: usize = 15;

/// Der echte Quell-Seam: liest die CSV-CAMT-v8-Datei am gehaltenen Pfad. Der Pfad wird erst beim
/// [`fetch`](SourceSeam::fetch) berührt (bis dahin passiert kein I/O).
pub struct CsvCamtV8Source {
    path: PathBuf,
}

impl CsvCamtV8Source {
    /// Bindet den Seam an die CSV-CAMT-v8-Datei am gegebenen Pfad (die Realdatei liefert Markus
    /// später per CLI-Argument; die Tests reichen synthetische Fixtures nach echtem Header).
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl SourceSeam for CsvCamtV8Source {
    fn fetch(&self) -> Result<Vec<RawRecord>, SourceUnavailable> {
        read_records(&self.path)
    }
}

/// Öffnet die Datei (`;`-getrennt, mit Kopfzeile) und parst jede Datenzeile positionsbasiert zu einem
/// [`RawRecord`]. Ein Öffnungs-/Lese-/Parse-Fehler wird zum terminalen, **redigierten**
/// [`SourceUnavailable`] (io-Diagnose: Mechanik, nie Zellinhalt).
fn read_records(path: &Path) -> Result<Vec<RawRecord>, SourceUnavailable> {
    let mut reader = ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_path(path)
        .map_err(read_failed)?;

    reader
        .records()
        .map(|row| {
            row.map(|record| into_raw_record(&record))
                .map_err(read_failed)
        })
        .collect()
}

/// Bildet eine geparste Zeile positionsbasiert auf das Seam-DTO ab. Weder Präsenz- noch
/// Ein-Währung-Prüfung finden **hier** statt (beide liegen formatunabhängig in der Adapter-ACL,
/// `SourceBridge`) — der Reader liefert nur die rohen Zellen; eine leere Zelle wird zu `None`.
fn into_raw_record(record: &StringRecord) -> RawRecord {
    RawRecord {
        ordering_account: cell(record, COL_ORDERING_ACCOUNT),
        booking_day: cell(record, COL_BOOKING_DAY),
        value_date: cell(record, COL_VALUE_DATE),
        amount: cell(record, COL_AMOUNT),
        currency: cell(record, COL_CURRENCY),
        recipient_name: cell(record, COL_RECIPIENT_NAME),
        creditor_id: cell(record, COL_CREDITOR_ID),
        counter_account: cell(record, COL_COUNTER_ACCOUNT),
        purpose: cell(record, COL_PURPOSE),
        booking_text: cell(record, COL_BOOKING_TEXT),
    }
}

/// Liest die Zelle an `index`: getrimmt, leer/fehlend → `None` (der Domänen-`NonEmptyString` trimmt
/// stromabwärts ohnehin — so deckt sich „leer" hier mit „fehlt" dort).
fn cell(record: &StringRecord, index: usize) -> Option<String> {
    record
        .get(index)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// Reicht die io-Diagnose eines Lese-/Parse-Fehlers durch (Pfad/OS/Parse-Mechanik). **Redaktion:**
/// `csv::Error` benennt Position und Fehlerart, nie einen Zellinhalt — die Diagnose bleibt zudem
/// lokal (kein Egress).
fn read_failed(error: csv::Error) -> SourceUnavailable {
    SourceUnavailable {
        message: format!("csv source read failed: {error}"),
    }
}
