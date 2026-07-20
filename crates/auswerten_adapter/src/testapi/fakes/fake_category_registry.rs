use crate::abstractions::{CategoryRegistrySeam, RegistryUnavailable};

/// Fake des **Kategorie-Registers**: hält die Fingerabdrücke, die die Domäne bereits kennt, und
/// spielt so die vertagte persistente Ablage
/// ([Issue 0009](../../../../docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md))
/// für die Specs.
///
/// **Leer ist der Normalfall** und keine Notlösung: ein frischer Lauf kennt nichts, und dann geht
/// jede Buchungsart einmal ans Orakel. Wer die Lage „das kenne ich schon" braucht, arrangiert sie.
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeCategoryRegistry {
    known: Vec<String>,
    failure: Option<String>,
}

impl FakeCategoryRegistry {
    /// Arrangiert einen Fingerabdruck als **bekannt** — als hätte ein früherer Lauf sein Etikett
    /// bereits gelernt.
    pub(crate) fn insert(&mut self, fingerprint: &str) {
        self.known.push(fingerprint.to_string());
    }

    /// Lässt jede Auskunft terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure = Some(message.to_string());
    }
}

impl CategoryRegistrySeam for FakeCategoryRegistry {
    fn known_among(&self, candidates: &[String]) -> Result<Vec<String>, RegistryUnavailable> {
        if let Some(message) = self.failure.clone() {
            return Err(RegistryUnavailable { message });
        }
        // Nur die **gefragten** Kandidaten — genau die `WHERE fingerprint IN (…)`, die io später
        // formuliert; was das Register sonst noch führt, geht diesen Lauf nichts an.
        Ok(candidates
            .iter()
            .filter(|candidate| self.known.contains(candidate))
            .cloned()
            .collect())
    }
}
