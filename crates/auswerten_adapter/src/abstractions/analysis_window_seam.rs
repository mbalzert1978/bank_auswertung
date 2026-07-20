use crate::models::AnalysisWindowRow;

/// Der öffentliche **Lese-Seam** auf `fenstern`s Fenster-Tabelle: liefert das eingefrorene
/// Auswertungsfenster **zu einer ID** — samt der obfuskierten Buchungen, auf die es per
/// Fremdschlüssel zeigt (io: die Fenster-Zeile plus ihr Join auf die `obfuskiert`-Ablage).
/// **io erfüllt ihn** — roh-typisiert, ohne Domänen-Kenntnis. Die
/// `SourceBridge` mappt den von der Domäne verlangten Port
/// `auswerten_domain::AnalysisWindowSource` darauf.
///
/// **Der Join lebt hier, nicht im Handler:** die Domäne verlangt *ein* Aggregat zu *einer* ID; dass
/// io das aus zwei Tabellen zusammensetzt, ist Beschaffungs-Mechanik. Ein zweiter Port „hol mir jetzt
/// noch die Buchungen" würde die Zerlegung der Ablage in die Domäne tragen.
///
/// **Fehlbar und terminal, mit zwei unterscheidbaren Ausgängen:** ein `Err` ist der Fehlschlag der
/// **Beschaffung** ([`SourceUnavailable`]), `Ok(None)` die Auskunft „zu dieser ID liegt **nichts**".
/// Beides endet terminal, aber aus verschiedenen Gründen — sie zu einem überladenen `Option`
/// zusammenzuziehen verlöre die Diagnose.
pub trait AnalysisWindowSeam {
    /// Liest das Auswertungsfenster zur ID; `Ok(None)` = es gibt keines, `Err` = die Beschaffung
    /// misslang (terminal).
    fn read(&self, window_id: &str) -> Result<Option<AnalysisWindowRow>, SourceUnavailable>;
}

/// Die Beschaffung ist **misslungen** — terminaler Seam-Fehler, der die **io-Diagnose** trägt
/// (Beschaffungs-Mechanik: Query-/DB-Fehler); der Text bleibt lokal, kein Egress.
///
/// **Redaktions-Pflicht am Seam (Vertrag, nicht Konstruktion):** die Meldung beschreibt die
/// **Beschaffungs-Mechanik** und zitiert **nie** Zeilen-/Feldinhalt — anders als der statische
/// Feldname der Präsenz-Prüfung lässt sich das dem Typ nicht ansehen, io muss es beim Erfüllen
/// einhalten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnavailable {
    /// Die durchgereichte io-Diagnose (Beschaffungs-Mechanik, nie Zeilen-/Feldinhalt).
    pub message: String,
}

impl std::fmt::Display for SourceUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nur die Diagnose selbst — den terminalen Meldungssatz besitzt allein die Domänen-`Display`
        // von `EvaluationError::SourceUnavailable` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SourceUnavailable {}
