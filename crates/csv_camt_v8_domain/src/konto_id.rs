use crate::non_empty_string::NonEmptyString;

/// Obfuskierte Kennung eines eigenen Kontos (`KONTO_x`) — der Domänen-Wert, den
/// die Buchung zur Unterscheidung der eigenen Konten trägt, **nie** die echte
/// IBAN/Kontonummer. Die Vergabe besorgt der Obfuskierungs-Slice; die CSV-Domäne
/// erhält das Token bereits fertig und hält es nur.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KontoIdEntity(NonEmptyString);

impl KontoIdEntity {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
