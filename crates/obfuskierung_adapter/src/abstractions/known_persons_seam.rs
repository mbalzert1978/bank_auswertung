/// Der öffentliche **Namens-Seam**: liefert die bekannten Personen-Namen, die der Freitext-Engpass
/// im Verwendungszweck ersetzt. **io erfüllt ihn** — roh-typisiert, ohne Domänen-Kenntnis; die
/// `PersonsBridge` mappt den von der Domäne verlangten Port
/// `obfuskierung_domain::KnownPersonsSource` darauf (Strings → `KnownPersons`).
///
/// Welche Namen das sind, entscheidet der Erfüller: bei `io` die Empfänger-Namen aus dem
/// einlesen-Bestand plus die lokale Namensliste. Die Namen sind Klartext und bleiben **lokal** — sie
/// fließen in kein Egress-DTO, nur in die Ersetzung. Leere Einträge übergeht die Bridge.
///
/// **Fehlbar und terminal** ([`PersonsUnavailable`]): eine nicht beschaffbare Liste lässt die
/// Obfuskierung scheitern, statt still mit zu wenigen Namen zu scrubben.
pub trait KnownPersonsSeam {
    /// Die bekannten Personen-Namen, oder der terminale Beschaffungs-Fehler.
    fn known_persons(&self) -> Result<Vec<String>, PersonsUnavailable>;
}

/// Die Namensliste ließ sich **nicht beschaffen** — terminaler Seam-Fehler, der die **io-Diagnose**
/// trägt (Beschaffungs-Mechanik: Query-/DB-/Schema-Fehler). Die `PersonsBridge` reicht sie als
/// `reason` in den redigierten Domänen-Fehler `ObfuscationError::PersonsUnavailable` weiter.
///
/// **Redaktions-Pflicht am Seam (Vertrag, nicht Konstruktion):** die Meldung beschreibt die
/// **Beschaffungs-Mechanik** und zitiert **nie** einen Namen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonsUnavailable {
    /// Die durchgereichte io-Diagnose (Beschaffungs-Mechanik, nie ein Name).
    pub message: String,
}

impl std::fmt::Display for PersonsUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nur die Diagnose selbst — den terminalen Meldungssatz besitzt allein die Domänen-`Display`
        // von `ObfuscationError::PersonsUnavailable` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PersonsUnavailable {}
