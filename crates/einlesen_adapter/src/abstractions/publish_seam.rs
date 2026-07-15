use crate::commands::IngestedMessage;

/// Der öffentliche, mechanische **Publish-Seam**: reiht das ausgehende „Eingelesen"-Kommando
/// ([`IngestedMessage`] — nur die ID als Zeiger) beim nachgelagerten Slice ein. **io erfüllt ihn**
/// (der letzte Outbox-Hop) — roh-typisiert, ohne Domänen-Kenntnis. Er ist **fehlbar** und
/// **terminal**: ein misslungenes Einreihen ist ein [`PublishUnavailable`]. Die
/// [`IngestedPublisher`](crate::bridges::IngestedPublisher) mappt ihn auf den internen Domänen-Port
/// `einlesen_domain::EventSink`, indem sie die ID auf das [`IngestedMessage`] setzt und den Fehler
/// in den Domänen-Fehler hüllt.
pub trait PublishSeam {
    /// Reiht das „Eingelesen"-Kommando ein; `Err` = das Einreihen scheiterte terminal. Der Commit
    /// besorgt io.
    fn publish(&self, message: IngestedMessage) -> Result<(), PublishUnavailable>;
}

/// Das Veröffentlichen ist **misslungen** — terminaler Seam-Fehler, der die **io-Diagnose** trägt
/// (Enqueue-Mechanik, nie Satz-/Feldinhalt); der Text bleibt lokal, kein Egress.
///
/// **Redaktions-Pflicht am Seam (Vertrag, nicht Konstruktion):** die Meldung beschreibt die
/// **Enqueue-Mechanik** und zitiert **nie** Satz-/Feldinhalt; io muss das beim Erfüllen einhalten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishUnavailable {
    /// Die durchgereichte io-Diagnose (Enqueue-Mechanik, nie Satz-/Feldinhalt).
    pub message: String,
}

impl std::fmt::Display for PublishUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nur die Diagnose selbst — den terminalen Meldungssatz besitzt allein die
        // Domänen-`Display` von `SinkError::PublishFailed` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PublishUnavailable {}
