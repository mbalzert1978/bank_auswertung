use auswerten_domain::{AnalysisWindow, AnalysisWindowSource, EvaluationError, WindowId};

use crate::abstractions::{AnalysisWindowSeam, SourceUnavailable};

/// Die **Port-Bridge** für den Lese-Port: erfüllt `auswerten_domain::AnalysisWindowSource` und mappt
/// ihn auf den öffentlichen [`AnalysisWindowSeam`] (den io erfüllt). „außen → innen": die opake
/// Domänen-[`WindowId`] wird zum rohen String des Seams, die gelieferte Zeile zum Domänen-Aggregat —
/// inklusive der **Präsenz-Prüfung** (ACL), deren fehlender Feldname redigiert in den terminalen
/// `EvaluationError::MissingMandatoryField` wandert.
///
/// Sie schließt hier die **drei** Ausgänge des Seams zu den zwei terminalen Domänen-Fehlern: ein
/// `Err` → `SourceUnavailable` (io-Diagnose durchgereicht), ein `Ok(None)` →
/// `WindowNotFound` mit der **opaken ID** (kein Rohwert).
pub(crate) struct SourceBridge<'a> {
    pub(crate) seam: &'a dyn AnalysisWindowSeam,
}

impl AnalysisWindowSource for SourceBridge<'_> {
    fn read(&self, window: &WindowId) -> Result<AnalysisWindow, EvaluationError> {
        self.seam
            .read(window.as_str())
            .map_err(
                |SourceUnavailable { message }| EvaluationError::SourceUnavailable {
                    reason: message,
                },
            )?
            .ok_or_else(|| EvaluationError::WindowNotFound(window.clone()))?
            .to_window()
            .map_err(EvaluationError::from)
    }
}
