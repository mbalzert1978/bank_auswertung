/// Ein getrimmter, garantiert nicht-leerer Text — der eine validierende Engpass,
/// den die obfuskierten Token-Value-Objects dieses Slice teilen. Crate-intern und
/// **ohne serde** (die Domäne ist dependency-frei); die Leer-Prüfung lebt an
/// genau einer Stelle.
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
