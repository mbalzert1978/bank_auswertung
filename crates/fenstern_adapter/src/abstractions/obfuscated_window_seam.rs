use crate::models::ObfuscatedRow;

/// Der öffentliche **Lese-Seam** auf die `obfuskiert`-Ablage: liefert die Zeilen einer Seite
/// (io: `WHERE booking_day BETWEEN from AND to`, auf dem sortierbaren Tag-Stamp aus ADR-0013,
/// Entscheidung C). **io erfüllt ihn** — roh-typisiert (`i64`), ohne Domänen-Kenntnis. Die
/// [`SourceBridge`](crate::adapter::SourceBridge) mappt den von der Domäne verlangten Port
/// `fenstern_domain::ObfuscatedWindowSource` darauf.
///
/// **Fehlbar und terminal:** ein Fehlschlag der Beschaffung ist **nie Abwesenheit**, sondern ein
/// [`SourceUnavailable`] — darum ein `Result`, kein überladenes `Option::None`. `Ok(vec![])` ist
/// eine **gültige** Antwort (die Seite ist leer); dass daraus kein Fenster wird, entscheidet die
/// Konstruktion des Aggregats, nicht der Seam.
pub trait ObfuscatedWindowSeam {
    /// Liest die obfuskierten Zeilen der Spanne `[from … to]` (beide Grenzen einschließlich, in
    /// Lesereihenfolge); `Err` = Beschaffung misslungen (terminal).
    fn read(&self, from: i64, to: i64) -> Result<Vec<ObfuscatedRow>, SourceUnavailable>;
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
        // von `WindowError::SourceUnavailable` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SourceUnavailable {}
