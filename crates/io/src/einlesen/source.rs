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
//! **Zeilen-Schlüssel und Buchungsstand** ([ADR-0016](../../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)):
//! zwei Dinge leitet der Reader zusätzlich ab, weil sie **Format-Wissen** sind und darum hierher
//! gehören, nicht in den Slice:
//!
//! - den **Zeilen-Schlüssel** je Zeile — Auftragskonto + Kundenreferenz (End-to-End) + Betrag, wenn
//!   die Kundenreferenz gefüllt ist; sonst **alle** Spalten. Dazu kommt die **laufende Nummer**
//!   gleicher Zeilen innerhalb dieser Datei, damit zwei echte, gleiche Buchungen am selben Tag beide
//!   zählen. Abgelegt wird nur ein **Digest** (SipHash-1-3, 128 Bit, fester Schlüssel), nie die
//!   Bestandteile im Klartext;
//! - den **Buchungsstand** aus der `Info`-Spalte: `Umsatz vorgemerkt` → [`RecordStatus::Pending`],
//!   alles andere → gebucht. Überspringen tut die Zeile erst die ACL des Slice.
//!
//! **Reines Lesen/Parsen — keine fachliche Zusicherung hier.** Der Reader liefert nur die rohen
//! `RawRecord`s. Die **formatunabhängigen** Inbound-Zusicherungen (eine Währung über den ganzen
//! Batch **und** die Präsenz der Pflichtfelder) liegen bewusst **nicht** im CSV-Reader, sondern in
//! der Adapter-ACL (`SourceBridge` hinter dem Seam) — so erbt sie jedes spätere Quellformat
//! (MT940/XML). Eine leere Zelle wird zu `None` (der Domänen-`NonEmptyString` trimmt ohnehin). Nur
//! ein **Lese-/Parse-Fehler** ist hier terminal: er trägt die durchgereichte io-Diagnose
//! (Pfad/OS/Parse-Mechanik, nie Zellinhalt).

use std::collections::HashMap;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

use csv::{ReaderBuilder, StringRecord};
use einlesen_adapter::{RawRecord, RecordStatus, SourceSeam, SourceUnavailable};
use siphasher::sip128::{Hasher128, SipHasher13};

// Positionen der verwerteten Spalten im Standard-Sparkassen-Header (0-indexiert). Die BIC (Spalte 13)
// fehlt bewusst — sie wird nie eingelesen und bleibt io-lokal.
const COL_ORDERING_ACCOUNT: usize = 0;
const COL_BOOKING_DAY: usize = 1;
const COL_VALUE_DATE: usize = 2;
const COL_BOOKING_TEXT: usize = 3;
const COL_PURPOSE: usize = 4;
const COL_CREDITOR_ID: usize = 5;
const COL_END_TO_END: usize = 7;
const COL_RECIPIENT_NAME: usize = 11;
const COL_COUNTER_ACCOUNT: usize = 12;
const COL_AMOUNT: usize = 14;
const COL_CURRENCY: usize = 15;
const COL_INFO: usize = 16;

/// Der `Info`-Wert, mit dem die Bank eine **vorgemerkte** (noch nicht gebuchte) Zeile kennzeichnet.
const PENDING_INFO: &str = "Umsatz vorgemerkt";

/// Der SEPA-Platzhalter für „keine Kundenreferenz angegeben" — er zählt wie eine leere Zelle, sonst
/// teilten sich alle Zahlungen ohne Referenz eines Kontos **einen** Schlüssel.
const END_TO_END_PLACEHOLDER: &str = "NOTPROVIDED";

/// Der **feste** Schlüssel des Zeilen-Digests. Bewusst **nicht** der Seed: hinge der Zeilen-Schlüssel
/// am Seed, erkennte nach einem Seed-Wechsel kein Import mehr eine schon eingelesene Zeile — der
/// Bestand verdoppelte sich still. Der Digest soll nichts verbergen (die Zeile liegt lokal ohnehin
/// roh in `einlesen_lines`), sondern nur die Kundenreferenz und die zusammengesetzte Zeile nicht im
/// Klartext ablegen.
const ROW_KEY_HASH_KEYS: (u64, u64) = (0x7a65_696c_656e_2d73, 0x6368_6c75_6573_7365);

/// Unterscheidet die beiden Schlüssel-Arten im Digest, damit ein Kundenreferenz-Schlüssel nie mit
/// einem Alle-Spalten-Schlüssel zusammenfallen kann. Die angehängte Ziffer ist die Version der
/// Ableitung — ändert sie sich je, erkennt kein Import mehr eine früher eingelesene Zeile.
const END_TO_END_LABEL: &str = "kundenreferenz/1";
const ALL_COLUMNS_LABEL: &str = "alle-spalten/1";

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

    let rows = reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .map_err(read_failed)?;
    Ok(rows
        .iter()
        .zip(row_keys(&rows))
        .map(|(record, row_key)| into_raw_record(record, row_key))
        .collect())
}

/// Bildet eine geparste Zeile positionsbasiert auf das Seam-DTO ab. Weder Präsenz- noch
/// Ein-Währung-Prüfung finden **hier** statt (beide liegen formatunabhängig in der Adapter-ACL,
/// `SourceBridge`) — der Reader liefert nur die rohen Zellen; eine leere Zelle wird zu `None`.
fn into_raw_record(record: &StringRecord, row_key: String) -> RawRecord {
    RawRecord {
        row_key: Some(row_key),
        status: status(record),
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

/// Der Buchungsstand der Zeile aus ihrer `Info`-Spalte: allein `Umsatz vorgemerkt` ist vorgemerkt,
/// alles andere (auch eine fehlende Spalte) gebucht.
fn status(record: &StringRecord) -> RecordStatus {
    match record.get(COL_INFO).map(str::trim) {
        Some(info) if info.eq_ignore_ascii_case(PENDING_INFO) => RecordStatus::Pending,
        _ => RecordStatus::Booked,
    }
}

/// Die Zeilen-Schlüssel aller Zeilen der Datei, in Lesereihenfolge. Je Zeile ihre **Identität**
/// ([`identity`]) plus die laufende Nummer, die gleiche Identitäten **in dieser Datei** durchzählt:
/// die erste gleiche Zeile trägt 1, die zweite 2. So zählen zwei echte, gleiche Buchungen beide, und
/// ein überlappender Auszug, der dieselben Zeilen in derselben Reihenfolge wieder trägt, erzeugt
/// dieselben Schlüssel.
///
/// **Die Grenze** (ADR-0016): die Nummer zählt innerhalb **einer** Datei. Schneidet ein Export einen
/// Tag mittendrin ab, trägt der nächste Export die übrigen gleichen Zeilen jenes Tages ab Nummer 1 —
/// und sie sähen aus wie die schon bekannten. Ein Export darf darum keinen Tag mittendrin abschneiden.
fn row_keys(rows: &[StringRecord]) -> Vec<String> {
    rows.iter()
        .scan(HashMap::<Vec<String>, u64>::new(), |seen, row| {
            let identity = identity(row);
            let occurrence = seen.entry(identity.clone()).or_insert(0);
            *occurrence += 1;
            Some(digest(&identity, *occurrence))
        })
        .collect()
}

/// Die **Identität** einer Zeile, aus der ihr Schlüssel entsteht — ein Etikett gefolgt von den
/// Bestandteilen:
///
/// - mit gefüllter **Kundenreferenz** (End-to-End): Auftragskonto + Kundenreferenz + Betrag. Der
///   Betrag steht dabei, weil eine Rücklastschrift die Kundenreferenz der ursprünglichen Lastschrift
///   wiederholt — ohne ihn verschluckte ein späterer Auszug die Rückbuchung als „schon bekannt";
/// - **ohne** sie: alle Spalten der Zeile (das Auftragskonto ist die erste).
fn identity(row: &StringRecord) -> Vec<String> {
    let trimmed = |index| row.get(index).map(str::trim).unwrap_or_default();
    let reference = trimmed(COL_END_TO_END);
    let has_reference =
        !reference.is_empty() && !reference.eq_ignore_ascii_case(END_TO_END_PLACEHOLDER);
    if has_reference {
        [
            END_TO_END_LABEL,
            trimmed(COL_ORDERING_ACCOUNT),
            reference,
            trimmed(COL_AMOUNT),
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    } else {
        std::iter::once(ALL_COLUMNS_LABEL)
            .chain(row.iter().map(str::trim))
            .map(str::to_string)
            .collect()
    }
}

/// Der Digest aus Identität und laufender Nummer: 32 Hex-Zeichen (128 Bit). Jeder Bestandteil geht
/// **längen-präfixiert** ein, damit verschobene Zellgrenzen (`"ab";"c"` gegen `"a";"bc"`) nicht
/// denselben Digest ergeben. Die 128 Bit halten eine zufällige Kollision — die eine echte Buchung
/// still als „schon bekannt" verwürfe — praktisch aus.
fn digest(identity: &[String], occurrence: u64) -> String {
    let (key0, key1) = ROW_KEY_HASH_KEYS;
    let mut hasher = SipHasher13::new_with_keys(key0, key1);
    identity.iter().for_each(|part| {
        hasher.write_u64(u64::try_from(part.len()).unwrap_or(u64::MAX));
        hasher.write(part.as_bytes());
    });
    hasher.write_u64(occurrence);
    format!("{:032x}", hasher.finish128().as_u128())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline_test_support::csv_source;

    /// Die Zeilen-Schlüssel einer Fixture in Lesereihenfolge (eine fehlende Zelle wäre ein Fehler
    /// des Readers und bleibt als `None` sichtbar).
    fn schluessel(fixture: &str) -> Result<Vec<Option<String>>, SourceUnavailable> {
        Ok(csv_source(fixture)
            .fetch()?
            .into_iter()
            .map(|record| record.row_key)
            .collect())
    }

    /// Sind alle Schlüssel vorhanden und paarweise verschieden?
    fn paarweise_verschieden(keys: &[Option<String>]) -> bool {
        let vorhanden: Vec<&String> = keys.iter().flatten().collect();
        let einmalig: std::collections::HashSet<&&String> = vorhanden.iter().collect();
        vorhanden.len() == keys.len() && einmalig.len() == keys.len()
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn dieselbe_datei_ergibt_bei_jedem_lesen_dieselben_schluessel() -> Result<(), SourceUnavailable>
    {
        // Genau darauf ruht „ein neuer Import ergänzt": derselbe Auszug, ein zweites Mal gelesen,
        // muss jede Zeile unter demselben Schlüssel wiedererkennen lassen.
        let erstes = schluessel("lauf.csv")?;
        let zweites = schluessel("lauf.csv")?;

        assert_eq!(erstes, zweites);
        assert!(paarweise_verschieden(&erstes), "{erstes:?}");
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn zwei_gleiche_zeilen_ohne_kundenreferenz_bekommen_verschiedene_schluessel()
    -> Result<(), SourceUnavailable> {
        // Zweimal dasselbe Parkticket am selben Tag: zwei echte Buchungen, Spalte für Spalte gleich.
        // Erst die laufende Nummer unterscheidet sie — sonst zählte nur eine.
        let keys = schluessel("zwei_gleiche.csv")?;

        assert_eq!(keys.len(), 2);
        assert!(paarweise_verschieden(&keys), "{keys:?}");
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ruecklastschrift_und_platzhalter_referenz_verschmelzen_nicht()
    -> Result<(), SourceUnavailable> {
        // Lastschrift und ihre Rücklastschrift teilen die Kundenreferenz, nicht den Betrag; zwei
        // Überweisungen mit dem SEPA-Platzhalter `NOTPROVIDED` teilen keine echte Referenz. Alle vier
        // sind verschiedene Buchungen und brauchen verschiedene Schlüssel.
        let keys = schluessel("kundenreferenz.csv")?;

        assert_eq!(keys.len(), 4);
        assert!(paarweise_verschieden(&keys), "{keys:?}");
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn der_schluessel_ist_ein_digest_ohne_iban_und_ohne_kundenreferenz()
    -> Result<(), SourceUnavailable> {
        let keys = schluessel("kundenreferenz.csv")?;

        for key in keys.iter().flatten() {
            assert_eq!(key.len(), 32, "{key}");
            assert!(key.chars().all(|c| c.is_ascii_hexdigit()), "{key}");
            for rohwert in ["DE12100000000000000200", "E2E-VB-2607", "VEREIN"] {
                assert!(!key.contains(rohwert), "{key} trägt {rohwert}");
            }
        }
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_vorgemerkte_zeile_wird_als_vorgemerkt_gemeldet() -> Result<(), SourceUnavailable> {
        // Gemeldet, nicht verworfen: überspringen tut sie die ACL des Slice.
        let status: Vec<RecordStatus> = csv_source("vorgemerkt.csv")
            .fetch()?
            .into_iter()
            .map(|record| record.status)
            .collect();

        assert_eq!(status, vec![RecordStatus::Pending, RecordStatus::Booked]);
        Ok(())
    }
}
