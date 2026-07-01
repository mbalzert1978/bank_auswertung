use crate::NonEmptyString;

/// Stabiles Ersatz-Token für eine Privatperson (`PERSON_07`). Die
/// Klartext-Zuordnung bleibt in einem separaten, lokalen Mapping-Store
/// außerhalb der Domäne (ADR-0006) — die Domäne kennt nur das Token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pseudonym(NonEmptyString);

impl Pseudonym {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn inner(&self) -> &NonEmptyString {
        &self.0
    }
}

impl From<NonEmptyString> for Pseudonym {
    fn from(text: NonEmptyString) -> Self {
        Self(text)
    }
}
