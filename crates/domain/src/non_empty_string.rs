/// Ein getrimmter, garantiert nicht-leerer Text — der eine validierende
/// Engpass, den alle nicht-leeren String-Value-Objects teilen. Jeder solche
/// Newtype wird per `From<NonEmptyString>` daraus gebildet; die Leer-Prüfung
/// lebt damit an genau einer Stelle (und alloziert genau einmal).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        let trimmed = value.as_ref().trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
