use crate::NonEmptyString;

/// Die SEPA-Gläubiger-ID: öffentlicher Firmen-Schlüssel, identifiziert die
/// Firma, nicht die Privatperson — bewusster Analyse-Anker, der die
/// Obfuskierung übersteht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GläubigerId(NonEmptyString);

impl GläubigerId {
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

impl From<NonEmptyString> for GläubigerId {
    fn from(text: NonEmptyString) -> Self {
        Self(text)
    }
}
