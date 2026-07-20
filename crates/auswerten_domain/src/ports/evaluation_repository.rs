use crate::entities::WindowEvaluation;
use crate::error::EvaluationError;

/// Der von der Domäne **verlangte** Ablage-Port: legt (staged) das Pro-Fenster-Ergebnis in die
/// **eigene** Tabelle des Slice (mit Fremdschlüssel auf das Auswertungsfenster). Der Handler
/// *staged* nur; den atomaren Commit besorgt io.
///
/// **Ohne Rückgabe der neu vergebenen ID, bewusst:** die Ablage vergibt sie (io:
/// `INSERT … RETURNING id`), aber diese Reichweite publiziert **keinen** Zeiger stromabwärts — es
/// gibt heute keinen Konsumenten dafür. Eine ID zurückzugeben, die niemand liest, wäre ein
/// Vertrags-Versprechen auf Vorrat; der Port wächst, wenn der Empfänger da ist.
///
/// **Fehlbar und terminal** ([`EvaluationError`]).
pub trait EvaluationRepository {
    /// Reiht das Ergebnis ein (Staging), oder scheitert terminal.
    fn save(&self, evaluation: &WindowEvaluation) -> Result<(), EvaluationError>;
}
