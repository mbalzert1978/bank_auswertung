use crate::{GläubigerId, NonEmptyString, Pseudonym};

/// Die Gegenpartei einer Buchung — trägt das wichtigste Kategorie-Signal.
/// `Firma` identifiziert über die Gläubiger-ID, `Person` über ein stabiles
/// Pseudonym; ein Klarname ist in keinem der beiden Fälle enthalten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Empfänger {
    Firma(GläubigerId),
    Person(Pseudonym),
}

impl Empfänger {
    /// Fail-safe Auflösung „Firma vs. Privatperson" (siehe CONTEXT.md):
    /// vorhandene Gläubiger-ID → `Firma`; fehlt sie, ist der Fall unsicher
    /// und wird zum Schutz als `Person` behandelt — der teurere Fehler wäre
    /// ein geleakter Klarname, nicht ein Pseudonym zu viel. `pseudonym`
    /// liefert der Aufrufer für genau diesen Unsicherheits-Fall.
    pub fn resolve(gläubiger_id: Option<GläubigerId>, pseudonym: Pseudonym) -> Self {
        match gläubiger_id {
            Some(id) => Self::Firma(id),
            None => Self::Person(pseudonym),
        }
    }

    pub(crate) fn identity_key(&self) -> &NonEmptyString {
        match self {
            Self::Firma(id) => id.inner(),
            Self::Person(pseudonym) => pseudonym.inner(),
        }
    }
}
