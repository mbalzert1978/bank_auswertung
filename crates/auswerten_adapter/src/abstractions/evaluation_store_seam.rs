use crate::models::WindowEvaluationRow;

/// Der öffentliche **Speicher-Seam** der Ergebnis-Tabelle: legt (staged) eine
/// [`WindowEvaluationRow`] ab (io: `INSERT`). **io erfüllt ihn** (sqlx) — den Commit besorgt io.
///
/// **Ohne Rückgabe der vergebenen ID:** die Ablage vergibt sie, aber diese Reichweite publiziert
/// **keinen** Zeiger stromabwärts (siehe `auswerten_domain::EvaluationRepository`) — der Seam
/// verspricht darum nur, was gebraucht wird.
///
/// **Fehlbar und terminal:** ein Scheitern ist ein seam-lokaler [`StoreUnavailable`]; die
/// [`RepositoryBridge`](crate::adapter::RepositoryBridge) mappt ihn „außen → innen" auf den
/// terminalen Domänen-Fehler (ACL).
pub trait EvaluationStoreSeam {
    /// Legt die Ergebnis-Zeile ab; `Err` = der Zugriff scheiterte terminal.
    fn save(&self, evaluation: &WindowEvaluationRow) -> Result<(), StoreUnavailable>;
}

/// Die Ablage ist **misslungen** — terminaler Seam-Fehler, der die **io-Diagnose** trägt
/// (Zugriffs-Mechanik: DB-/Transaktions-Fehler); der Text bleibt lokal, kein Egress.
///
/// **Redaktions-Pflicht am Seam (Vertrag, nicht Konstruktion):** die Meldung beschreibt die
/// **Zugriffs-Mechanik** und zitiert **nie** Zeilen-/Feldinhalt; io muss das beim Erfüllen einhalten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreUnavailable {
    /// Die durchgereichte io-Diagnose (Zugriffs-Mechanik, nie Zeilen-/Feldinhalt).
    pub message: String,
}

impl std::fmt::Display for StoreUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nur die Diagnose selbst — den terminalen Meldungssatz besitzt allein die Domänen-`Display`
        // von `EvaluationError::StorageUnavailable` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StoreUnavailable {}
