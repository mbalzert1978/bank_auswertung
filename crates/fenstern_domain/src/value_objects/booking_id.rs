use crate::value_objects::NonEmptyString;

/// Die **Identität** einer obfuskierten Buchung — der Fremdschlüssel, den das
/// [`AnalysisWindow`](crate::AnalysisWindow) einfriert (das Aggregat referenziert Fremd-Entitäten
/// **nur per ID**, ADR-0012). Opak: der Slice deutet sie nie, er hält sie fest.
///
/// **Nicht-Leere im Typ, kein Guard:** anders als die gleichnamige ID des Obfuskierungs-Slice und
/// `einlesen`s `IngestionId` — die einen rohen Envelope-/Store-String entgegennehmen und die
/// Nicht-Leere darum **hart behaupten** müssen (`assert!`) — entsteht diese ID **ausschließlich**
/// aus einem bereits geprüften [`NonEmptyString`]: die Präsenz-Prüfung des Adapters (ACL) weist eine
/// leere Zelle schon vorher **terminal und redigiert** ab (`WindowError::MissingMandatoryField`).
/// Deshalb ist der Leerwert hier **unkonstruierbar** statt „geprüft" (ADR-0005) — ein `assert!` wäre
/// ein Pfad, der nie feuern kann, und würde einen sauberen Fehler-Ausgang durch einen Panic ersetzen.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BookingId(String);

impl BookingId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Der **einzige** Weg zur ID: aus dem geprüften Kern. Damit trägt der Typ die Nicht-Leere als
/// Beweis, statt sie zur Laufzeit zu behaupten.
impl From<NonEmptyString> for BookingId {
    fn from(value: NonEmptyString) -> Self {
        Self(value.as_str().to_string())
    }
}
