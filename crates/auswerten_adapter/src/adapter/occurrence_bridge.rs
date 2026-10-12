use auswerten_domain::{EvaluationError, OccurrenceSource, RunHistory, RunSpan};

use crate::abstractions::{OccurrenceSeam, SourceUnavailable};

/// Die **Port-Bridge** der fenster-übergreifenden Reichweite: erfüllt
/// `auswerten_domain::OccurrenceSource` und mappt ihn auf den öffentlichen [`OccurrenceSeam`] (den
/// io erfüllt). „außen → innen": die [`RunSpan`] wird zu den beiden rohen Tag-Stamps des Seams, die
/// gelieferte Historie zur Domänen-[`RunHistory`] — inklusive der **Präsenz-Prüfung** (ACL), deren
/// fehlender Feldname **redigiert** in den terminalen `EvaluationError::MissingMandatoryField`
/// wandert.
///
/// Sie schließt die **zwei** Ausgänge des Seams zu den zwei terminalen Domänen-Fehlern: ein `Err` →
/// `SourceUnavailable` (io-Diagnose durchgereicht), eine unvollständige Zeile →
/// `MissingMandatoryField` (nur der statische Feldname). Eine **leere** Historie ist **kein**
/// Fehler — es gibt hier keinen Zeiger, der ins Leere zeigen könnte, nur eine Spanne, in der nichts
/// liegt.
pub(crate) struct OccurrenceBridge<'a> {
    pub(crate) seam: &'a dyn OccurrenceSeam,
}

impl OccurrenceSource for OccurrenceBridge<'_> {
    fn history_in(&self, span: RunSpan) -> Result<RunHistory, EvaluationError> {
        self.seam
            .history_in(span.from().value(), span.to().value())
            .map_err(
                |SourceUnavailable { message }| EvaluationError::SourceUnavailable {
                    reason: message,
                },
            )?
            .to_history()
            .map_err(|missing| EvaluationError::MissingMandatoryField {
                field: missing.field(),
            })
    }
}
