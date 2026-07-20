use crate::models::AnalysisWindowRow;

/// Der öffentliche **Speicher-Seam** der Fenster-Tabelle: legt (staged) eine [`AnalysisWindowRow`]
/// ab und **vergibt dabei die Identität** — er gibt die zugeteilte ID (Textform) zurück (io:
/// `INSERT … RETURNING id`). **io erfüllt ihn** (sqlx) — die Tabelle ist die Wahrheit für
/// stromabwärts; den Commit (Tabelle **und** die ausgehenden Nachrichten atomar in **einer**
/// Arbeitseinheit) besorgt io.
///
/// **Fehlbar und terminal:** ein Scheitern ist ein seam-lokaler [`StoreUnavailable`]; die
/// `RepositoryBridge` mappt ihn „außen → innen" auf den
/// terminalen Domänen-Fehler (ACL).
pub trait AnalysisWindowStoreSeam {
    /// Legt die Fenster-Zeile ab und gibt die dabei vergebene ID zurück; `Err` = der Zugriff
    /// scheiterte terminal.
    fn save(&self, window: &AnalysisWindowRow) -> Result<String, StoreUnavailable>;
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
        // von `WindowError::StorageUnavailable` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StoreUnavailable {}
