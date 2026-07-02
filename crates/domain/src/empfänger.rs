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
    /// Fail-safe Auflösung „Firma vs. Privatperson" (siehe CONTEXT.md) — die eine
    /// Stelle, die diese Datenschutz-Regel trägt: vorhandene Gläubiger-ID →
    /// `Firma`; fehlt sie, ist der Fall unsicher und wird zum Schutz als `Person`
    /// behandelt — der teurere Fehler wäre ein geleakter Klarname, nicht ein
    /// Pseudonym zu viel.
    ///
    /// `pseudonym` wird **lazy** ausgewertet und nur im Person-Fall abgefragt,
    /// damit eine Firma keinen Pseudonym-Zähler verbraucht. Liefert die Quelle
    /// kein Pseudonym (`None` — z. B. weil auch kein Name vorliegt), ist der
    /// Empfänger nicht identifizierbar und die Auflösung ergibt `None`.
    pub fn resolve(
        gläubiger_id: Option<GläubigerId>,
        pseudonym: impl FnOnce() -> Option<Pseudonym>,
    ) -> Option<Self> {
        match gläubiger_id {
            Some(id) => Some(Self::Firma(id)),
            None => pseudonym().map(Self::Person),
        }
    }

    pub(crate) fn identity_key(&self) -> &NonEmptyString {
        match self {
            Self::Firma(id) => id.inner(),
            Self::Person(pseudonym) => pseudonym.inner(),
        }
    }
}
