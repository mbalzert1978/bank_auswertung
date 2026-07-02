/// Ein getrimmter, garantiert nicht-leerer Text — der eine validierende Engpass,
/// den die nicht-leeren String-Wert-Typen dieses Slice teilen. Crate-intern; die
/// Leer-Prüfung lebt damit an genau einer Stelle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct NonEmptyString(String);

impl NonEmptyString {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub(crate) fn new(value: impl AsRef<str>) -> Option<Self> {
        let trimmed = value.as_ref().trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
