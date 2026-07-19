use std::fmt;

/// Warum ein Schritt des Slice terminal scheitert — **die Pipeline ist durchweg terminal**
/// (ADR-0012): **kein** Dead-Letter, **kein** Retry im Slice. Lesen, Ablegen und Publizieren sind
/// **je fehlbar**, und jede ihrer Fehlvarianten endet hier.
///
/// **Redaktion:** keine Variante trägt je einen Rohwert. Die Invarianten-Brüche
/// ([`EmptyWindow`](WindowError::EmptyWindow)/[`MixedCurrency`](WindowError::MixedCurrency)) sind
/// **feldlos**, die Präsenz-Verletzung trägt **nur den statischen Feldnamen**, und die
/// Mechanik-Fehlschläge tragen die **io-Diagnose** (Zugriffs-/Enqueue-Mechanik), zu deren
/// Inhaltsfreiheit sich der jeweilige Seam vertraglich verpflichtet.
///
/// Die Domäne ist dependency-frei: `Display`/`Error` sind von Hand geschrieben, kein `thiserror`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowError {
    /// Die obfuskierten Zeilen der Seite ließen sich nicht beschaffen. Trägt die io-Diagnose
    /// (Beschaffungs-Mechanik, nie Satz-/Feldinhalt).
    SourceUnavailable { reason: String },
    /// Eine gelesene Zeile hat ein leeres/fehlendes Pflichtfeld — trägt **nur** den statischen
    /// Feldnamen, nie den Rohwert.
    MissingMandatoryField { field: &'static str },
    /// Die Seite trägt keine einzige Buchung — ein Fenster ohne Daten ist unkonstruierbar
    /// (ADR-0013, Entscheidung B: jedes Fenster sitzt auf echten Daten). **Feldlos.**
    EmptyWindow,
    /// Die Buchungen der Seite tragen mehr als eine Währung — die Domäne summiert keine gemischten
    /// Währungen (ADR-0005). **Feldlos:** welche Währungen kollidierten, ist Dateninhalt.
    MixedCurrency,
    /// Die Ablage des Auswertungsfensters ist gescheitert. Trägt die io-Diagnose (Zugriffs-Mechanik).
    StorageUnavailable { reason: String },
    /// Das Einreihen einer ausgehenden Nachricht ist gescheitert. Trägt die io-Diagnose
    /// (Enqueue-Mechanik).
    PublishFailed { reason: String },
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowError::SourceUnavailable { reason } => {
                write!(
                    f,
                    "could not read obfuscated bookings of the page: {reason}"
                )
            }
            WindowError::MissingMandatoryField { field } => {
                write!(f, "mandatory field missing in obfuscated booking: {field}")
            }
            WindowError::EmptyWindow => {
                write!(f, "no booking on this page: an analysis window needs data")
            }
            WindowError::MixedCurrency => {
                write!(f, "page bookings do not share a single currency")
            }
            WindowError::StorageUnavailable { reason } => {
                write!(f, "could not store the analysis window: {reason}")
            }
            WindowError::PublishFailed { reason } => {
                write!(f, "could not enqueue the outgoing message: {reason}")
            }
        }
    }
}

impl std::error::Error for WindowError {}
