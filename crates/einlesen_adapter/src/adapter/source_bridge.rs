use einlesen_domain::{RawLine, RawLineSource, RunConfiguration, SourceError};

use crate::abstractions::{SourceSeam, SourceUnavailable};
use crate::models::{RawRecord, RecordStatus};

/// Die **Port-Bridge** für den Beschaffungs-Port: erfüllt `einlesen_domain::RawLineSource`
/// und mappt ihn auf den öffentlichen [`SourceSeam`] (den io erfüllt). „außen → innen": der
/// Seam liefert rohe [`RawRecord`](crate::models::RawRecord)s, die Bridge bildet sie
/// **batch-atomar** auf Domänen-`RawLine`s ab.
///
/// **Hier, hinter dem Seam, liegen die beiden formatunabhängigen Inbound-Zusicherungen der
/// ACL** (jedes Quellformat — CSV heute, MT940/XML später — erbt sie, weil sie *nicht* im
/// jeweiligen io-Reader sitzen):
/// - **Ein-Währung** über den ganzen Batch (die Domäne summiert keine gemischten Währungen):
///   ein Bruch wird redigiert auf `SourceError::MixedCurrency` abgebildet, bevor der Handler
///   eine Zeile deutet.
/// - **Vorgemerktes überspringen** (seit [ADR-0016](../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)):
///   eine Zeile mit [`RecordStatus::Pending`] fällt **vor** allen Prüfungen heraus — sie bricht
///   nichts ab und zählt nicht.
/// - **Eindeutige Zeilen-Schlüssel** innerhalb des Batches (ADR-0016): ein Doppel wird redigiert auf
///   `SourceError::DuplicateRowKey` abgebildet.
/// - **Präsenz** der Pflichtfelder (seit [Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
///   hier statt in der Consume-Bridge): das erste fehlende Pflichtfeld wird auf
///   `SourceError::MissingMandatoryField` (nur statischer Feldname) abgebildet.
///
/// Ein `Err` des Seams ist **terminal** (nie Abwesenheit) → `SourceError::Unavailable` mit
/// durchgereichter io-Diagnose. Die (vorerst leere) [`RunConfiguration`] verwirft die Bridge —
/// der Seam bleibt arglos, bis der io-Schritt entscheidet, was sie trägt. Vorbild: die
/// `SourceBridge` des Obfuskierungs-Slice.
pub(crate) struct SourceBridge<'a> {
    pub(crate) seam: &'a dyn SourceSeam,
}

impl RawLineSource for SourceBridge<'_> {
    fn fetch(&self, _configuration: &RunConfiguration) -> Result<Vec<RawLine>, SourceError> {
        let records: Vec<RawRecord> = self
            .seam
            .fetch()
            .map_err(|SourceUnavailable { message }| SourceError::Unavailable { reason: message })?
            .into_iter()
            // Vorgemerktes zuerst und vor jeder Prüfung verwerfen: eine vorgemerkte Zeile ist noch
            // keine Buchung — leere Valuta, Platzhalter statt IBAN — und würde sonst die ganze Datei
            // an der Pflichtfeld-Prüfung scheitern lassen (ADR-0016).
            .filter(|record| record.status == RecordStatus::Booked)
            .collect();
        ensure_single_currency(&records)?;
        ensure_unique_row_keys(&records)?;
        records
            .iter()
            .map(|record| {
                RawLine::try_from(record).map_err(|missing| SourceError::MissingMandatoryField {
                    field: missing.field(),
                })
            })
            .collect()
    }
}

/// Sichert zu, dass **keine zwei** Zeilen des Batches denselben Zeilen-Schlüssel tragen — der
/// Vertrag der Quelle (gleiche Zeilen unterscheidet ihre laufende Nummer, ADR-0016). Ein Doppel ließe
/// die Ablage eine echte Buchung still als „schon bekannt" verwerfen; darum terminal, redigiert
/// ([`SourceError::DuplicateRowKey`], datenlos). Fehlende Schlüssel zählen hier nicht — sie scheitern
/// stromabwärts an der Präsenz-Prüfung.
fn ensure_unique_row_keys(records: &[RawRecord]) -> Result<(), SourceError> {
    let mut seen = std::collections::HashSet::new();
    records
        .iter()
        .filter_map(|record| record.row_key.as_deref())
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .all(|key| seen.insert(key))
        .then_some(())
        .ok_or(SourceError::DuplicateRowKey)
}

/// Sichert **eine** Währung über den ganzen Batch zu — formatunabhängige Inbound-Zusicherung der
/// ACL (die Domäne summiert keine gemischten Währungen). Nur **nicht-leere** Währungszellen
/// zählen (leere/fehlende scheitern ohnehin stromabwärts an der Präsenz-Prüfung); weichen sie
/// voneinander ab, ist der Batch terminal → redigiertes [`SourceError::MixedCurrency`] (datenlos,
/// nie ein Code). Der Vergleich ist **case-insensitiv** — dieselbe Deutung wie der spätere
/// Währungs-Decode über den `Currencies`-Port, damit `eur`/`EUR` kein falscher Bruch ist.
fn ensure_single_currency(records: &[RawRecord]) -> Result<(), SourceError> {
    let mut currencies = records
        .iter()
        .filter_map(|record| record.currency.as_deref())
        .map(str::trim)
        .filter(|currency| !currency.is_empty());
    match currencies.next() {
        None => Ok(()),
        Some(first) => currencies
            .all(|currency| currency.eq_ignore_ascii_case(first))
            .then_some(())
            .ok_or(SourceError::MixedCurrency),
    }
}
