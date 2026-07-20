use auswerten_domain::{CategoryRegistry, EvaluationError, Fingerprint};

use crate::abstractions::{CategoryRegistrySeam, RegistryUnavailable};

/// Die **Port-Bridge** zum Kategorie-Register: erfüllt `auswerten_domain::CategoryRegistry` und
/// setzt die Frage auf die rohen Schlüssel-Zeichenketten des [`CategoryRegistrySeam`] um (ACL). Ein
/// `Err` des Seams ist **terminal** → `EvaluationError::RegistryUnavailable`.
pub(crate) struct RegistryBridge<'a> {
    pub(crate) seam: &'a dyn CategoryRegistrySeam,
}

impl CategoryRegistry for RegistryBridge<'_> {
    fn known_among(&self, candidates: &[Fingerprint]) -> Result<Vec<Fingerprint>, EvaluationError> {
        let known =
            self.seam
                .known_among(
                    &candidates
                        .iter()
                        .map(|fingerprint| fingerprint.as_str().to_string())
                        .collect::<Vec<_>>(),
                )
                .map_err(|RegistryUnavailable { message }| {
                    EvaluationError::RegistryUnavailable { reason: message }
                })?;
        // Zurück kommen die **hereingereichten** Kandidaten, gefiltert — nicht aus den Zeichenketten
        // des Seams neu gebildete Schlüssel. Ein Fingerabdruck entsteht ausschließlich als reine
        // Domänenfunktion aus einer Gegenpartei; gäbe es hier einen zweiten Weg, wäre er keine
        // Aussage der Domäne mehr. Ein Schlüssel, den das Register führt, nach dem aber niemand
        // gefragt hat, geht dabei folgerichtig ins Leere.
        Ok(candidates
            .iter()
            .filter(|candidate| known.iter().any(|entry| entry == candidate.as_str()))
            .cloned()
            .collect())
    }
}
