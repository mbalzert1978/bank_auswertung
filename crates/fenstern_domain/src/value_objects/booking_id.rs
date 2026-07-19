/// Die **Identität** einer obfuskierten Buchung — der Fremdschlüssel, den das
/// [`AnalysisWindow`](crate::AnalysisWindow) einfriert (das Aggregat referenziert Fremd-Entitäten
/// **nur per ID**, ADR-0012). Opak: der Slice deutet sie nie, er hält sie fest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BookingId(String);

impl BookingId {
    /// Baut die ID aus ihrem Rohwert (der Lese-Seam reicht die Ablage-ID herein). Bewusst **opak**
    /// (kein `NonEmptyString`, keine fehlbare Signatur): die ID vergibt die Persistenz, nicht der
    /// Slice. Die **Präsenz** erzwingt der Adapter (ACL) an der Zeile, bevor sie hier ankommt —
    /// darum nimmt dieser Konstruktor sie als bereits geprüft entgegen.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
