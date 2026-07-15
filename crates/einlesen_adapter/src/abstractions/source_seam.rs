use crate::dto::RawRecord;

/// Der öffentliche **Quell-Seam** — der in
/// [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md) dokumentierte
/// „source seam": beschafft den rohen Batch der Quelle. **io erfüllt ihn** (CSV-CAMT-v8 ist
/// **eine** Eingabeform dahinter; XML-camt/MT940 später = weitere io-Adapter, kein neuer
/// Slice) — roh-typisiert, ohne Domänen-Kenntnis, spricht extern das [`RawRecord`]-DTO. Die
/// [`SourceBridge`](crate::bridges::SourceBridge) mappt den von der Domäne verlangten Port
/// `einlesen_domain::RawLineSource` darauf (Records → Domänen-`RawLine`s, Präsenz-Prüfung).
///
/// Ein Fehlschlag der Beschaffung ist **nie Abwesenheit**, sondern ein terminaler Fehler —
/// darum ein `Result`, kein überladenes `Option::None` (die `Option`-Form des
/// `IngestedSourceSeam` im Obfuskierungs-Slice kodiert eine fehlende Zeile, die dort ein
/// gebrochenes Invariant ist — ebenfalls terminal, aber ein anderer Ausgang als der hiesige
/// Beschaffungs-Fehlschlag). `Ok(vec![])` = leerer
/// Batch (gültig, es passiert nichts). Seit [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)
/// **öffentlich** (wie `StoreSeam`), damit `io` ihn erfüllt.
pub trait SourceSeam {
    /// Beschafft die rohen Zeilen der Quelle (in Lesereihenfolge); `Err` = Beschaffung
    /// misslungen (terminal).
    fn fetch(&self) -> Result<Vec<RawRecord>, SourceUnavailable>;
}

/// Die Beschaffung ist **misslungen** — terminaler Seam-Fehler, der die **io-Diagnose**
/// trägt: io speist den Text seines eigenen Fehler-Results ein (Pfad, OS-/Parse-Fehler),
/// damit nachvollziehbar bleibt, was genau schieflief; der Text bleibt lokal, kein Egress.
///
/// **Redaktions-Pflicht am Seam (Vertrag, nicht Konstruktion):** die Meldung beschreibt die
/// **Beschaffungs-Mechanik** und zitiert **nie** Satz-/Feldinhalt — anders als der statische
/// Feldname der Präsenz-Prüfung lässt sich das dem Typ nicht ansehen, io muss es beim
/// Erfüllen einhalten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnavailable {
    /// Die durchgereichte io-Diagnose (Beschaffungs-Mechanik, nie Satz-/Feldinhalt).
    pub message: String,
}

impl std::fmt::Display for SourceUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nur die Diagnose selbst — den terminalen Meldungssatz besitzt allein die
        // Domänen-`Display` von `SourceError::Unavailable` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SourceUnavailable {}
