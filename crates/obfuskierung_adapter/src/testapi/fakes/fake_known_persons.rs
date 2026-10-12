use super::ArrangedFailure;
use crate::abstractions::{KnownPersonsSeam, PersonsUnavailable};

/// Fake des Namens-Seams: hält die arrangierten Personen-Namen im Speicher und spielt so die
/// io-seitige Namensliste (einlesen-Empfänger + lokale Liste) für die Specs. Ohne Arrangement ist die
/// Liste leer.
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Beschaffungs-Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeKnownPersons {
    names: Vec<String>,
    failure: ArrangedFailure,
}

impl FakeKnownPersons {
    /// Arrangiert einen bekannten Personen-Namen.
    pub(crate) fn insert(&mut self, name: &str) {
        self.names.push(name.to_string());
    }

    /// Lässt jede Beschaffung terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure.arm(message);
    }
}

impl KnownPersonsSeam for FakeKnownPersons {
    fn known_persons(&self) -> Result<Vec<String>, PersonsUnavailable> {
        self.failure
            .check(|message| PersonsUnavailable { message })?;
        Ok(self.names.clone())
    }
}
