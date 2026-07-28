use std::collections::HashMap;

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
    failure: Option<String>,
}

impl FakeIngestedSource {
    /// Arrangiert eine Roh-Zeile unter ihrer ID (die der Handler später liest).
    pub(crate) fn insert(&mut self, id: &str, row: IngestedRow) {
        self.rows.insert(id.to_string(), row);
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

impl IngestedSourceSeam for FakeIngestedSource {
    fn read(&self, id: &str) -> Result<Option<IngestedRow>, SourceUnavailable> {
        self.arranged_failure()?;
        Ok(self.rows.get(id).cloned())
    }
}
