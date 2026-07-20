use crate::models::{CategoryAnswerRow, CategoryRequestRow};

/// Der öffentliche **Orakel-Seam**: reicht die Anfragen der unbekannten Buchungsarten hinaus und
/// nimmt die Etiketten entgegen. **io erfüllt ihn** — später als Export → Chat → Import
/// ([Issue 0011](../../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md)); dass dahinter
/// ein LLM steckt, weiß dieser Seam so wenig wie die Domäne.
///
/// **Der Stapel ist die Operation**, nicht eine Optimierung: das Orakel ist ein Chat. Je Art einen
/// eigenen Austausch zu führen wäre nicht dieselbe Sache in klein.
///
/// **Fehlbar und terminal:** ein Scheitern ist ein seam-lokaler [`CategorizerUnavailable`]; die
/// `CategorizerBridge` mappt ihn „außen → innen" auf den
/// terminalen Domänen-Fehler (ACL).
pub trait CategorizerSeam {
    /// Fragt die Kategorien an; `Err` = der Zugriff scheiterte terminal.
    fn ask(
        &self,
        requests: &[CategoryRequestRow],
    ) -> Result<Vec<CategoryAnswerRow>, CategorizerUnavailable>;
}

/// Das Orakel war **nicht zu erreichen** — terminaler Seam-Fehler mit der io-Diagnose.
///
/// **Redaktions-Pflicht am Seam (Vertrag, nicht Konstruktion):** die Meldung beschreibt die
/// **Zugriffs-Mechanik** und zitiert **nie** den angefragten Text oder eine Antwort; io muss das
/// beim Erfüllen einhalten. Gerade hier wiegt das schwer: dieser Seam ist der eine, an dem Daten
/// die Maschine verlassen — seine Fehlermeldung darf nicht zum Rückkanal werden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizerUnavailable {
    /// Die durchgereichte io-Diagnose (Zugriffs-Mechanik, nie Text oder Antwort).
    pub message: String,
}

impl std::fmt::Display for CategorizerUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nur die Diagnose selbst — den terminalen Meldungssatz besitzt allein die Domänen-`Display`
        // von `EvaluationError::CategorizerUnavailable` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CategorizerUnavailable {}
