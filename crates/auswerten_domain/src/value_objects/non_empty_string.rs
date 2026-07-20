/// Der **eine** validierende Kern der Zeichenketten-Wert-Objekte des Slice: getrimmt und
/// garantiert nicht-leer (_parse, don't validate_ — ADR-0005). Alles, was darauf aufbaut
/// ([`Currency`](crate::Currency)), wird per `From` daraus gebaut und hat die Prüfung damit
/// **hinter** sich; sie wiederholt sich nirgends.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// Baut den Wert aus einem Rohtext: getrimmt; leer/nur-Whitespace → `None`. Die einzige
    /// Stelle, an der Leere abgewiesen wird.
    #[must_use]
    pub fn new(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
