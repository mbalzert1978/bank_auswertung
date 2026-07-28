use std::collections::HashMap;

use super::ArrangedFailure;
use crate::abstractions::{IngestedSourceSeam, SourceUnavailable};
use crate::models::IngestedRow;

/// Fake des einlesen-Lese-Seams: hält die arrangierten Roh-Zeilen je ID im Speicher und spielt so
/// den vertagten einlesen-Speicher (Roh-Tabelle) für die Specs. `read` liefert die Zeile zur ID —
/// eine **unbekannte** ID liefert `Ok(None)` (der Zeiger zeigt ins Leere), was die Bridge in den
/// terminalen `NotFound` mappt. Vorbild: die Fakes der öffentlichen Abstractions des Einlese-Slice.
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Beschaffungs-Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeIngestedSource {
    rows: HashMap<String, IngestedRow>,
    failure: ArrangedFailure,
}

impl FakeIngestedSource {
    /// Arrangiert eine Roh-Zeile unter ihrer ID (die der Handler später liest).
    pub(crate) fn insert(&mut self, id: &str, row: IngestedRow) {
        self.rows.insert(id.to_string(), row);
    }

    /// Lässt jede Beschaffung terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure.arm(message);
    }
}

impl IngestedSourceSeam for FakeIngestedSource {
    fn read(&self, id: &str) -> Result<Option<IngestedRow>, SourceUnavailable> {
        self.failure
            .check(|message| SourceUnavailable { message })?;
        Ok(self.rows.get(id).cloned())
    }
}
