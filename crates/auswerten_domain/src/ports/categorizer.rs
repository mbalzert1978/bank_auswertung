use crate::error::EvaluationError;
use crate::value_objects::{CategoryAnswer, CategoryRequest};

/// Der von der Domäne **verlangte** Port zum **Orakel** (ADR-0004): er bildet die der Domäne
/// **unbekannten** Fingerabdrücke vom obfuskierten Text auf eine Kategorie ab. Dahinter steht
/// später ein LLM (Export → Chat → Import); dass es eines ist, weiß dieser Port nicht.
///
/// **Orakel, kein Pipeline-Schritt.** Der Aufruf ist eine **Domänen-Entscheidung** — deshalb hängt
/// er hier am Handler und nicht an einem eigenen Consumer mit eigenem Outbox-Kommando. Der
/// Lackmustest von ADR-0004: nimmt man diesen Port heraus, rechnet der Slice weiter — Cashflow,
/// Fingerabdrücke, Gegenkonten stehen unverändert; es fehlt allein das automatische Etikett.
///
/// **Er fragt in einem Zug, nicht je Buchungsart einzeln:** die Anfragen kommen als **Stapel**
/// herein, weil das Orakel ein Chat ist und nicht ein Nachschlagewerk — je Art einen Austausch zu
/// führen wäre nicht dieselbe Operation in klein, sondern eine andere.
///
/// **Die Antwort ordnet der Adapter über den Schlüssel zu, den er selbst gestellt hat.** Ein
/// Etikett zu einem Fingerabdruck, nach dem nie gefragt wurde, ist keine Antwort — und ein Orakel
/// kann keine Schlüssel erfinden, die die Domäne dann glauben müsste.
///
/// **Fehlbar und terminal** (ADR-0012): kein Retry, kein Dead-Letter im Slice.
pub trait Categorizer {
    /// Fragt die Kategorien der **unbekannten** Buchungsarten an. Die Antwort darf **kürzer** sein
    /// als die Frage: für eine Art, die auch das Orakel nicht deuten kann, kommt schlicht kein
    /// Etikett zurück — eine Lücke ist eine gültige Auskunft, kein Fehlschlag.
    fn ask(&self, requests: &[CategoryRequest]) -> Result<Vec<CategoryAnswer>, EvaluationError>;
}
