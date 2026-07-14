use einlesen_domain::{RawLine, RawLineSource, RunConfiguration, SourceError};

use crate::abstractions::{SourceSeam, SourceUnavailable};

/// Die **Port-Bridge** für den Beschaffungs-Port: erfüllt `einlesen_domain::RawLineSource`
/// und mappt ihn auf den öffentlichen [`SourceSeam`] (den io erfüllt). „außen → innen": der
/// Seam liefert rohe [`RawRecord`](crate::dto::RawRecord)s, die Bridge bildet sie
/// **batch-atomar** auf Domänen-`RawLine`s ab — **hier, hinter dem Seam, liegt die
/// Präsenz-Prüfung** (seit [Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
/// nicht mehr in der Consume-Bridge): das erste fehlende Pflichtfeld wird (wie ein
/// io-Beschaffungsfehler) auf `SourceError::MissingMandatoryField` abgebildet, bevor der
/// Handler eine einzige Zeile deutet. Ein `Err` des Seams ist **terminal** (nie Abwesenheit)
/// → `SourceError::Unavailable` mit durchgereichter io-Diagnose. Die (vorerst leere)
/// [`RunConfiguration`] verwirft die Bridge — der Seam bleibt arglos, bis der io-Schritt
/// entscheidet, was sie trägt. Vorbild: die `SourceBridge` des Obfuskierungs-Slice.
pub(crate) struct SourceBridge<'a> {
    pub(crate) seam: &'a dyn SourceSeam,
}

impl RawLineSource for SourceBridge<'_> {
    fn fetch(&self, _configuration: &RunConfiguration) -> Result<Vec<RawLine>, SourceError> {
        self.seam
            .fetch()
            .map_err(|SourceUnavailable { message }| SourceError::Unavailable { reason: message })?
            .iter()
            .map(|record| {
                RawLine::try_from(record).map_err(|missing| SourceError::MissingMandatoryField {
                    field: missing.field(),
                })
            })
            .collect()
    }
}
