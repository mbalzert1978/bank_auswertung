/// Der öffentliche **Register-Seam**: beantwortet, welche der übergebenen Fingerabdrücke das
/// **Kategorie-Register** bereits kennt (D5, [ADR-0004](../../../../docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)).
/// **io erfüllt ihn** — die persistente Ablage samt gelernten Korrekturen baut
/// [Issue 0009](../../../../docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md); bis
/// dahin trägt ihn in den Specs ein Fake.
///
/// **Er spricht rohe Schlüssel-Zeichenketten**, keine Domänen-Fingerabdrücke: der Seam ist die
/// mechanische Außenseite, und das Register ist eine Tabelle mit einer Schlüsselspalte. Dass aus
/// den zurückgegebenen Zeichenketten wieder Domänen-Schlüssel werden, besorgt die
/// `RegistryBridge` — indem sie **gegen die selbst gestellten
/// Kandidaten matcht**, nie durch Neubildung aus einem String.
///
/// **Fehlbar und terminal:** ein Scheitern reist als [`RegistryUnavailable`] „außen → innen".
pub trait CategoryRegistrySeam {
    /// Gibt die Teilmenge der Kandidaten zurück, die das Register führt (io: `SELECT … WHERE
    /// fingerprint IN (…)`).
    fn known_among(&self, candidates: &[String]) -> Result<Vec<String>, RegistryUnavailable>;
}

/// Das Register war **nicht zu befragen** — terminaler Seam-Fehler mit der io-Diagnose.
///
/// **Redaktions-Pflicht am Seam (Vertrag, nicht Konstruktion):** die Meldung beschreibt die
/// **Zugriffs-Mechanik** und zitiert nie einen Schlüssel oder ein Etikett.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryUnavailable {
    /// Die durchgereichte io-Diagnose (Zugriffs-Mechanik, nie Zeilen-/Feldinhalt).
    pub message: String,
}

impl std::fmt::Display for RegistryUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nur die Diagnose selbst — den Meldungssatz besitzt die Domänen-`Display` von
        // `EvaluationError::RegistryUnavailable`.
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RegistryUnavailable {}
