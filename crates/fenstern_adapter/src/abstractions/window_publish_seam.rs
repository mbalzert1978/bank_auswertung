use crate::models::{HistoryCompleteMessage, WindowClosedMessage};

/// Der öffentliche, mechanische **Publish-Seam** des Slice: reiht die ausgehenden Nachrichten beim
/// nachgelagerten Slice ein. **io erfüllt ihn** (der letzte Outbox-Hop) — roh-typisiert, ohne
/// Domänen-Kenntnis.
///
/// **Ein Seam, mehrere Ausgänge:** der Fenster-Schritt publiziert in **derselben** Arbeitseinheit
/// mehr als eine Nachricht (ADR-0013, Entscheidung B) — sie gehen alle in **dieselbe** Outbox,
/// darum trägt ein Seam sie mit je einer Methode je Nachrichtentyp. Die Domänen-Ports bleiben
/// getrennt (je ein Sink-Port), die Bridges teilen sich diesen Seam.
///
/// **Fehlbar und terminal:** ein misslungenes Einreihen ist ein [`PublishUnavailable`]; die
/// Bridges mappen ihn auf den terminalen Domänen-Fehler.
pub trait WindowPublishSeam {
    /// Reiht den `WindowClosed`-Zeiger stromabwärts nach `auswerten` ein; `Err` = terminal. Der
    /// Commit besorgt io.
    fn publish_closed(&self, message: WindowClosedMessage) -> Result<(), PublishUnavailable>;

    /// Reiht den `HistoryComplete`-Abschluss mit der Lauf-Spanne ein; `Err` = terminal. Er committet
    /// atomar mit der Ablage und dem `WindowClosed`-Zeiger — dieselbe Arbeitseinheit.
    fn publish_history_complete(
        &self,
        message: HistoryCompleteMessage,
    ) -> Result<(), PublishUnavailable>;
}

/// Das Veröffentlichen ist **misslungen** — terminaler Seam-Fehler, der die **io-Diagnose** trägt
/// (Enqueue-Mechanik, nie Zeilen-/Feldinhalt); der Text bleibt lokal, kein Egress.
///
/// **Redaktions-Pflicht am Seam (Vertrag, nicht Konstruktion):** die Meldung beschreibt die
/// **Enqueue-Mechanik** und zitiert **nie** Zeilen-/Feldinhalt; io muss das beim Erfüllen einhalten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishUnavailable {
    /// Die durchgereichte io-Diagnose (Enqueue-Mechanik, nie Zeilen-/Feldinhalt).
    pub message: String,
}

impl std::fmt::Display for PublishUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nur die Diagnose selbst — den terminalen Meldungssatz besitzt allein die Domänen-`Display`
        // von `WindowError::PublishFailed` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PublishUnavailable {}
