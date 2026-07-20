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

    /// Der **jüngste Buchungstag des ganzen Bestands** (io: `SELECT MAX(booking_day)`) — der
    /// Initial-Anker, mit dem der **Anstoß** einen Lauf eröffnet (ADR-0013, Entscheidung D). Er ist
    /// **datenrelativ**: der Lauf startet am jüngsten vorhandenen Tag, nie an einer Uhr — darum
    /// braucht weder io noch der Spec eine Zeitquelle.
    ///
    /// `Ok(None)` ist die **gültige** Antwort „der Bestand ist leer" und beendet den Lauf
    /// erfolgreich, ohne dass etwas publiziert wird; `Err` = die Abfrage misslang (terminal).
    fn latest(&self) -> Result<Option<i64>, SourceUnavailable>;

    /// Der **jüngste Buchungstag echt unterhalb** von `day` (io: `SELECT MAX(booking_day) WHERE
    /// booking_day < ?`) — dieselbe Query-Art wie der Initial-Anker, nur mit Untergrenze. Er trägt
    /// die **Selbst-Fortsetzung und die Terminierung** des Pagers (ADR-0013, Entscheidung B,
    /// Präzisierung 2).
    ///
    /// `Ok(None)` ist die **gültige** Antwort „darunter liegt nichts mehr" und beendet die Kette;
    /// `Err` = die Abfrage misslang (terminal). Beides zu trennen ist der Grund für
    /// `Result<Option<_>>` statt eines überladenen `Option`.
    fn latest_before(&self, day: i64) -> Result<Option<i64>, SourceUnavailable>;
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
