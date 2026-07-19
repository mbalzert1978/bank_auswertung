use crate::error::WindowError;
use crate::models::AdvanceWindow;

/// Der von der Domäne **verlangte** Sink-Port für die **Selbst-Fortsetzung**: veröffentlicht
/// (staged) das nächste [`AdvanceWindow`], wenn unter der Fensterunterkante noch ein echter
/// Buchungstag liegt (ADR-0013, Entscheidung B, Präzisierung 2).
///
/// Er ist der **Gegenzweig** zu [`HistoryCompletePublisher`](crate::HistoryCompletePublisher):
/// **genau einer von beiden** feuert je Fenster-Schritt — nie beide, nie keiner. Die Kette
/// terminiert garantiert, weil der Anker dabei **streng monoton sinkt** (er liegt echt unter der
/// Unterkante der gerade verarbeiteten Seite).
///
/// **Fehlbar und terminal** ([`WindowError`]).
pub trait AdvanceWindowPublisher {
    /// Reiht den nächsten Fenster-Schritt ein (Staging), oder scheitert terminal.
    fn publish(&self, command: &AdvanceWindow) -> Result<(), WindowError>;
}
