use std::cell::RefCell;

use crate::abstractions::{CategorizerSeam, CategorizerUnavailable};
use crate::models::{CategoryAnswerRow, CategoryRequestRow};

/// Fake des **Orakels**: er hält fest, **wonach** gefragt wurde, und antwortet aus einem arrangierten
/// Vokabular Fingerabdruck → Kategorie.
///
/// Beide Hälften sind Zusagen, die ein Spec beobachten muss: an den festgehaltenen Anfragen prüft
/// er, dass **nur Unbekanntes** und **je Art nur einmal** gefragt wird, an den Antworten, dass die
/// Zuordnung über den **Fingerabdruck** läuft.
///
/// Nach einem Fingerabdruck, den das Vokabular nicht führt, **antwortet er nicht** — das ist keine
/// Lücke im Fake, sondern die Lage „auch das Orakel weiß es nicht", die es real gibt.
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeCategorizer {
    vocabulary: Vec<(String, String)>,
    rogue: Vec<CategoryAnswerRow>,
    asked: RefCell<Vec<CategoryRequestRow>>,
    failure: Option<String>,
}

impl FakeCategorizer {
    /// Arrangiert, welche Kategorie das Orakel zu welchem Fingerabdruck liefern würde.
    pub(crate) fn knows(&mut self, fingerprint: &str, category: &str) {
        self.vocabulary
            .push((fingerprint.to_string(), category.to_string()));
    }

    /// Arrangiert ein Etikett, das der Fake **ungefragt** mitschickt — zu einem Fingerabdruck, nach
    /// dem niemand gefragt hat. Damit lässt sich das **übergriffige** Orakel spielen: eines, das
    /// Schlüssel erfindet statt zu antworten.
    pub(crate) fn answers_unasked(&mut self, fingerprint: &str, category: &str) {
        self.rogue.push(CategoryAnswerRow {
            fingerprint: fingerprint.to_string(),
            category: category.to_string(),
        });
    }

    /// Die gestellten Anfragen in Reihenfolge — das konkrete Egress-DTO am Seam.
    pub(crate) fn asked(&self) -> Vec<CategoryRequestRow> {
        self.asked.borrow().clone()
    }

    /// Lässt jede Anfrage terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure = Some(message.to_string());
    }
}

impl CategorizerSeam for FakeCategorizer {
    fn ask(
        &self,
        requests: &[CategoryRequestRow],
    ) -> Result<Vec<CategoryAnswerRow>, CategorizerUnavailable> {
        // **Vor** dem Festhalten: ein gescheiterter Zugriff ist keine gestellte Frage.
        if let Some(message) = self.failure.clone() {
            return Err(CategorizerUnavailable { message });
        }
        self.asked.borrow_mut().extend_from_slice(requests);
        Ok(requests
            .iter()
            .filter_map(|request| {
                self.vocabulary
                    .iter()
                    .find(|(fingerprint, _)| *fingerprint == request.fingerprint())
                    .map(|(fingerprint, category)| CategoryAnswerRow {
                        fingerprint: fingerprint.clone(),
                        category: category.clone(),
                    })
            })
            .chain(self.rogue.iter().cloned())
            .collect())
    }
}
