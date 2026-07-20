use crate::entities::AnalysisWindow;
use crate::error::EvaluationError;
use crate::value_objects::WindowId;

/// Der von der Domäne **verlangte** Lese-Port, über den der Handler das eingefrorene
/// Auswertungsfenster **per ID** aus `fenstern`s Ablage holt — er bekommt einen **Zeiger**, keine
/// eingebetteten Daten (uniformes ID-Handoff, ADR-0012). **io erfüllt ihn**; in den Specs trägt ihn
/// ein Fake.
///
/// **Fehlbar und terminal** ([`EvaluationError`]): sowohl ein Fehlschlag der Beschaffung als auch
/// ein **ins Leere zeigender** Zeiger enden terminal — darum ein `Result<AnalysisWindow>` statt
/// eines überladenen `Option::None`, das „nicht da" und „nichts zu tun" vermengte.
pub trait AnalysisWindowSource {
    /// Liest das Auswertungsfenster zur ID, oder scheitert terminal.
    fn read(&self, window: &WindowId) -> Result<AnalysisWindow, EvaluationError>;
}
