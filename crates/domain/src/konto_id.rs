use crate::NonEmptyString;

/// Obfuskierte Kennung eines eigenen Kontos (`KONTO_x`), von der Domäne nur
/// zur Unterscheidung gebraucht — nie die echte IBAN/Kontonummer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KontoId(NonEmptyString);

impl KontoId {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<NonEmptyString> for KontoId {
    fn from(text: NonEmptyString) -> Self {
        Self(text)
    }
}
