use std::cell::RefCell;

use crate::abstractions::{AnalysisWindowStoreSeam, StoreUnavailable};
use crate::models::AnalysisWindowRow;

/// Fake der Fenster-Tabelle: hält die gestageten [`AnalysisWindowRow`]-DTOs im Speicher und spielt
/// so die vertagte Persistenz für die Specs. Beobachtet wird das **Staging** — an den festgehaltenen
/// DTOs prüft ein Spec die Feld-Treue des ganzen Fensters am realen Durchlauf.
///
/// **Die vergebene ID** ist deterministisch (`fenster-1`, `fenster-2`, … in Staging-Reihenfolge, wie
/// die Fakes der Vorgänger-Slices) — der Fake spielt `INSERT … RETURNING id`, damit ein Spec den
/// publizierten Zeiger exakt gegen die neu vergebene ID prüfen kann.
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Ablage-Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeAnalysisWindowStore {
    windows: RefCell<Vec<AnalysisWindowRow>>,
    failure: Option<String>,
}

impl FakeAnalysisWindowStore {
    /// Die gestageten Fenster in Reihenfolge — das konkrete DTO am Seam.
    pub(crate) fn windows(&self) -> Vec<AnalysisWindowRow> {
        self.windows.borrow().clone()
    }

    /// Lässt jede Ablage terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure = Some(message.to_string());
    }

    /// Das arrangierte Scheitern als Vorbedingung — `Ok(())`, solange keines gesetzt ist.
    fn arranged_failure(&self) -> Result<(), StoreUnavailable> {
        self.failure
            .clone()
            .map_or(Ok(()), |message| Err(StoreUnavailable { message }))
    }
}

impl AnalysisWindowStoreSeam for FakeAnalysisWindowStore {
    fn save(&self, window: &AnalysisWindowRow) -> Result<String, StoreUnavailable> {
        self.arranged_failure()?;
        let mut windows = self.windows.borrow_mut();
        let id = format!("fenster-{}", windows.len() + 1);
        windows.push(window.clone());
        Ok(id)
    }
}
