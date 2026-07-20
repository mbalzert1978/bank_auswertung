use auswerten_domain::{EvaluationError, OccurrenceSource, RunSpan, WindowedBooking};

use crate::abstractions::{OccurrenceSeam, SourceUnavailable};
use crate::models::WindowBookingRow;

/// Die **Port-Bridge** der fenster-übergreifenden Reichweite: erfüllt
/// `auswerten_domain::OccurrenceSource` und mappt ihn auf den öffentlichen [`OccurrenceSeam`] (den
/// io erfüllt). „außen → innen": die [`RunSpan`] wird zu den beiden rohen Tag-Stamps des Seams, die
/// gelieferten Zeilen werden zu Domänen-Buchungen — inklusive der **Präsenz-Prüfung** (ACL), deren
/// fehlender Feldname **redigiert** in den terminalen `EvaluationError::MissingMandatoryField`
/// wandert.
///
/// Sie schließt die **zwei** Ausgänge des Seams zu den zwei terminalen Domänen-Fehlern: ein `Err` →
/// `SourceUnavailable` (io-Diagnose durchgereicht), eine unvollständige Zeile →
/// `MissingMandatoryField` (nur der statische Feldname). Ein **leeres** Ergebnis ist **kein**
/// Fehler — es gibt hier keinen Zeiger, der ins Leere zeigen könnte, nur eine Spanne, in der nichts
/// liegt.
pub(crate) struct OccurrenceBridge<'a> {
    pub(crate) seam: &'a dyn OccurrenceSeam,
}

impl OccurrenceSource for OccurrenceBridge<'_> {
    fn bookings_in(&self, span: RunSpan) -> Result<Vec<WindowedBooking>, EvaluationError> {
        self.seam
            .bookings_in(span.from().value(), span.to().value())
            .map_err(
                |SourceUnavailable { message }| EvaluationError::SourceUnavailable {
                    reason: message,
                },
            )?
            .iter()
            .map(WindowBookingRow::to_booking)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|missing| EvaluationError::MissingMandatoryField {
                field: missing.field(),
            })
    }
}
