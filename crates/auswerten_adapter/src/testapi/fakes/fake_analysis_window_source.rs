use std::collections::BTreeMap;

use crate::abstractions::{AnalysisWindowSeam, SourceUnavailable};
use crate::models::AnalysisWindowRow;

/// Fake des Lese-Seams: hält die arrangierten Auswertungsfenster **je ID** im Speicher und spielt so
/// die vertagte Fenster-Ablage für die Specs. `read` liefert genau das Fenster zur ID — eine
/// **unbekannte** ID liefert `Ok(None)` (der Zeiger zeigt ins Leere), was die Bridge in den
/// terminalen `WindowNotFound` mappt.
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Beschaffungs-Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeAnalysisWindowSource {
    windows: BTreeMap<String, AnalysisWindowRow>,
    failure: Option<String>,
}

impl FakeAnalysisWindowSource {
    /// Arrangiert ein eingefrorenes Auswertungsfenster unter seiner ID.
    pub(crate) fn insert(&mut self, window_id: &str, window: AnalysisWindowRow) {
        self.windows.insert(window_id.to_string(), window);
    }

    /// Lässt jede Beschaffung terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure = Some(message.to_string());
    }

    /// Das arrangierte Scheitern als Vorbedingung — `Ok(())`, solange keines gesetzt ist.
    fn arranged_failure(&self) -> Result<(), SourceUnavailable> {
        self.failure
            .clone()
            .map_or(Ok(()), |message| Err(SourceUnavailable { message }))
    }
}

impl AnalysisWindowSeam for FakeAnalysisWindowSource {
    fn read(&self, window_id: &str) -> Result<Option<AnalysisWindowRow>, SourceUnavailable> {
        self.arranged_failure()?;
        Ok(self.windows.get(window_id).cloned())
    }
}
