use auswerten_domain::{EvaluationError, OccurrenceSource, RunHistory, RunSpan};

use crate::abstractions::{OccurrenceSeam, SourceUnavailable};
use crate::models::RunScope;

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
///
/// **Der Lauf-Scope sitzt hier** ([ADR-0016](../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)):
/// die Bridge wird je Kommando gebaut und trägt dessen Lauf; sie legt ihn neben der Spanne an den
/// Seam. Das Scoping ist Beschaffungs-Mechanik — die Domäne verlangt „die Historie dieses Laufs" und
/// muss den Lauf dafür nicht kennen.
pub(crate) struct OccurrenceBridge<'a> {
    pub(crate) seam: &'a dyn OccurrenceSeam,
    pub(crate) run: &'a RunScope,
}

impl OccurrenceSource for OccurrenceBridge<'_> {
    fn history_in(&self, span: RunSpan) -> Result<RunHistory, EvaluationError> {
        self.seam
            .history_in(self.run.as_str(), span.from().value(), span.to().value())
            .map_err(
                |SourceUnavailable { message }| EvaluationError::SourceUnavailable {
                    reason: message,
                },
            )?
            .to_history()
    }
}
