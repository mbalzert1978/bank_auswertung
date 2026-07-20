use crate::value_objects::{Category, Fingerprint};

/// Die **eingehende** Antwort des Kategorisierer-Orakels — das schmale DTO der Richtung „herein"
/// (ADR-0002), Gegenstück zum [`CategoryRequest`](crate::CategoryRequest) und **nicht** dasselbe
/// Ding: ein gemeinsames DTO für beide Richtungen trüge in jeder Richtung ein Feld, das dort nichts
/// zu suchen hat.
///
/// Sie schlüsselt auf den **Fingerabdruck, nie auf den Text**. Das ist der Kern und keine
/// Kosmetik: käme die Antwort am Text zurück, wäre der Text die Zuordnungsachse — und damit ein
/// Rückweg vom Etikett auf den Freitext, aus dem es stammt. Über den Fingerabdruck gibt es diesen
/// Rückweg nicht.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CategoryAnswer {
    fingerprint: Fingerprint,
    category: Category,
}

impl CategoryAnswer {
    #[must_use]
    pub fn new(fingerprint: Fingerprint, category: Category) -> Self {
        Self {
            fingerprint,
            category,
        }
    }

    #[must_use]
    pub fn fingerprint(&self) -> &Fingerprint {
        &self.fingerprint
    }

    #[must_use]
    pub fn category(&self) -> &Category {
        &self.category
    }
}
