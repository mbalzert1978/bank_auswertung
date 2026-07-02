use crate::non_empty_string::NonEmptyString;

/// Die SEPA-Gläubiger-ID: öffentlicher Firmen-Schlüssel, identifiziert die Firma,
/// nicht die Privatperson — bewusster Analyse-Anker, der die Obfuskierung
/// übersteht (kein struktureller Identifikator einer Person).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GläubigerIdEntity(NonEmptyString);

impl GläubigerIdEntity {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Stabiles Ersatz-Token für eine Privatperson (`PERSON_x`). Die
/// Klartext-Zuordnung bleibt im Obfuskierungs-Slice (ADR-0006); die CSV-Domäne
/// erhält nur das fertige Token und hält es.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PseudonymEntity(NonEmptyString);

impl PseudonymEntity {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Die Gegenpartei einer Buchung — trägt das wichtigste Kategorie-Signal.
/// `Firma` identifiziert über die Gläubiger-ID, `Person` über ein stabiles
/// Pseudonym; ein Klarname ist in keinem der beiden Fälle enthalten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmpfängerEntity {
    Firma(GläubigerIdEntity),
    Person(PseudonymEntity),
}

impl EmpfängerEntity {
    /// Fail-safe Auflösung „Firma vs. Privatperson" (siehe CONTEXT.md) — die eine
    /// Stelle, die diese Datenschutz-Regel trägt: vorhandene Gläubiger-ID →
    /// `Firma`; fehlt sie, ist der Fall unsicher und wird zum Schutz als `Person`
    /// behandelt — der teurere Fehler wäre ein geleakter Klarname, nicht ein
    /// Pseudonym zu viel.
    ///
    /// `pseudonym` wird **lazy** ausgewertet und nur im Person-Fall abgefragt,
    /// damit eine Firma keinen Pseudonym-Zähler verbraucht. Liefert die Quelle
    /// kein Pseudonym (`None` — z. B. weil auch kein Name vorliegt), ist der
    /// Empfänger nicht identifizierbar und die Auflösung ergibt `None`. Der
    /// Adapter reicht die Rohdaten und eine obfuskierende Pseudonym-Quelle herein
    /// — die Weiche selbst bleibt Domänenlogik.
    pub fn resolve(
        gläubiger_id: Option<GläubigerIdEntity>,
        pseudonym: impl FnOnce() -> Option<PseudonymEntity>,
    ) -> Option<Self> {
        match gläubiger_id {
            Some(id) => Some(Self::Firma(id)),
            None => pseudonym().map(Self::Person),
        }
    }
}
