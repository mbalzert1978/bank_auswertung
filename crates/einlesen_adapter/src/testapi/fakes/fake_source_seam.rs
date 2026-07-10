use crate::abstractions::{SourceSeam, SourceUnavailable};
use crate::dto::RawRecord;

/// Fake des Quell-Seams: hält die arrangierten Roh-Zeilen (oder eine arrangierte
/// Ausfall-Diagnose) und spielt so die vertagte io-Beschaffung (CSV-Datei) für die Specs.
/// `fetch` liefert die Zeilen in Lesereihenfolge — oder, wenn ein Ausfall arrangiert ist,
/// `Err(SourceUnavailable)` mit genau dieser Diagnose (ein Fehlschlag der Beschaffung ist
/// **terminal**, nie Abwesenheit). Vorbild: die übrigen Fakes der öffentlichen Abstractions.
#[derive(Default)]
pub(crate) struct FakeSourceSeam {
    records: Vec<RawRecord>,
    unavailable: Option<String>,
}

impl FakeSourceSeam {
    /// Arrangiert eine Roh-Zeile in der Quelle (in Lesereihenfolge).
    pub(crate) fn add_record(&mut self, record: RawRecord) {
        self.records.push(record);
    }

    /// Arrangiert den Fehlschlag der Beschaffung mit der gegebenen io-Diagnose.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.unavailable = Some(message.to_string());
    }
}

impl SourceSeam for FakeSourceSeam {
    fn fetch(&self) -> Result<Vec<RawRecord>, SourceUnavailable> {
        match &self.unavailable {
            Some(message) => Err(SourceUnavailable {
                message: message.clone(),
            }),
            None => Ok(self.records.clone()),
        }
    }
}
