use einlesen_domain::{RawLine, RawLineSource, RunConfiguration, SourceError};

use crate::abstractions::{SourceSeam, SourceUnavailable};
use crate::models::RawRecord;

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
        let records = self.seam.fetch().map_err(|SourceUnavailable { message }| {
            SourceError::Unavailable { reason: message }
        })?;
        ensure_single_currency(&records)?;
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

/// Sichert **eine** Währung über den ganzen Batch zu — formatunabhängige Inbound-Zusicherung der
/// ACL (die Domäne summiert keine gemischten Währungen). Nur **nicht-leere** Währungszellen
/// zählen (leere/fehlende scheitern ohnehin stromabwärts an der Präsenz-Prüfung); weichen sie
/// voneinander ab, ist der Batch terminal → redigiertes [`SourceError::MixedCurrency`] (datenlos,
/// nie ein Code).
fn ensure_single_currency(records: &[RawRecord]) -> Result<(), SourceError> {
    let mut currencies = records
        .iter()
        .filter_map(|record| record.currency.as_deref())
        .map(str::trim)
        .filter(|currency| !currency.is_empty());
    match currencies.next() {
        None => Ok(()),
        Some(first) => currencies
            .all(|currency| currency == first)
            .then_some(())
            .ok_or(SourceError::MixedCurrency),
    }
}
