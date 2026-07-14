/// Ein getrimmter, garantiert nicht-leerer Text — der eine validierende Engpass, den die
/// Wert-Objekte dieses Slice teilen (parse, don't validate). **Ohne serde** (die Domäne ist
/// dependency-frei); die Leer-Prüfung lebt an genau einer Stelle.
///
/// Öffentlich, weil der Adapter an der „außen → innen"-Grenze die rohen Zellen als
/// `NonEmptyString` an die Domäne reicht (ACL) — genau wie `einlesen_domain::NonEmptyString`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    #[must_use]
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        let trimmed = value.as_ref().trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
