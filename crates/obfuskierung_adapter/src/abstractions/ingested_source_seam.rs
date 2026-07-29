use crate::models::IngestedRow;

/// Der öffentliche **Lese-Seam** auf den einlesen-Speicher (Roh-Tabelle): liefert die rohe Zeile zu
/// einer Aggregat-ID. **io erfüllt ihn** (liest die einlesen-Roh-Tabelle) — roh-typisiert, ohne
/// Domänen-Kenntnis. Die `SourceBridge` mappt den von der Domäne
/// verlangten Port `obfuskierung_domain::RawBookingSource` darauf (Zeile → Domänen-Aggregat).
///
/// **Fehlbar und terminal, mit zwei unterscheidbaren Ausgängen** (ADR-0012: **alle drei**
/// I/O-Schritte jedes Slice sind fehlbar): ein `Err` ist der Fehlschlag der **Beschaffung**
/// ([`SourceUnavailable`]), `Ok(None)` die Auskunft „zu dieser ID liegt **nichts**". Beides endet
/// terminal, aber aus verschiedenen Gründen — sie zu einem überladenen `Option` zusammenzuziehen
/// verlöre die Diagnose und meldete eine unerreichbare Datenbank als „referenzierte Buchung nicht
/// gefunden". Formgleich mit dem `auswerten`-Lese-Seam.
pub trait IngestedSourceSeam {
    /// Liest die rohe einlesen-Zeile zur ID; `Ok(None)` = es liegt keine vor, `Err` = die
    /// Beschaffung misslang (terminal).
    fn read(&self, id: &str) -> Result<Option<IngestedRow>, SourceUnavailable>;
}

/// Die Beschaffung ist **misslungen** — terminaler Seam-Fehler, der die **io-Diagnose** trägt
/// (Beschaffungs-Mechanik: Query-/DB-/Schema-Fehler). Die `SourceBridge` reicht sie als `reason` in
/// den redigierten Domänen-Fehler `ObfuscationError::SourceUnavailable` weiter, damit ein unerreichbarer
/// Lese-Speicher diagnostizierbar bleibt — lokal, **kein Egress**: in ein ausgehendes DTO fließt sie
/// nie.
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
        // von `ObfuscationError::SourceUnavailable` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SourceUnavailable {}
