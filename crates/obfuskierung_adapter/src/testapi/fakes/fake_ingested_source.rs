use std::collections::HashMap;

use crate::abstractions::IngestedSourceSeam;
use crate::dto::IngestedRow;

/// Fake des einlesen-Lese-Seams: hält die arrangierten Roh-Zeilen je ID im Speicher und spielt so
/// den vertagten einlesen-Speicher (Roh-Tabelle) für die Specs. `read` liefert die Zeile zur ID
/// oder `None`. Vorbild: die Fakes der öffentlichen Abstractions des Einlese-Slice.
#[derive(Default)]
pub(crate) struct FakeIngestedSource {
    rows: HashMap<String, IngestedRow>,
}

impl FakeIngestedSource {
    /// Arrangiert eine Roh-Zeile unter ihrer ID (die der Handler später liest).
    pub(crate) fn insert(&mut self, id: &str, row: IngestedRow) {
        self.rows.insert(id.to_string(), row);
    }
}

impl IngestedSourceSeam for FakeIngestedSource {
    fn read(&self, id: &str) -> Option<IngestedRow> {
        self.rows.get(id).cloned()
    }
}
