use std::cell::RefCell;

use crate::abstractions::{EvaluationStoreSeam, StoreUnavailable};
use crate::models::WindowEvaluationRow;

/// Fake der Ergebnis-Tabelle: hält die gestageten [`WindowEvaluationRow`]-DTOs im Speicher und
/// spielt so die vertagte Persistenz für die Specs. Beobachtet wird das **Staging** — an den
/// festgehaltenen DTOs prüft ein Spec die Feld-Treue des ganzen Ergebnisses am realen Durchlauf.
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Ablage-Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeEvaluationStore {
    evaluations: RefCell<Vec<WindowEvaluationRow>>,
    failure: Option<String>,
}

impl FakeEvaluationStore {
    /// Die gestageten Ergebnisse in Reihenfolge — das konkrete DTO am Seam.
    pub(crate) fn evaluations(&self) -> Vec<WindowEvaluationRow> {
        self.evaluations.borrow().clone()
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

impl EvaluationStoreSeam for FakeEvaluationStore {
    fn save(&self, evaluation: &WindowEvaluationRow) -> Result<(), StoreUnavailable> {
        self.arranged_failure()?;
        self.evaluations.borrow_mut().push(evaluation.clone());
        Ok(())
    }
}
