use crate::entities::AnalysisWindow;
use crate::error::WindowError;
use crate::value_objects::WindowId;

/// Der von der Domäne **verlangte** Ablage-Port: legt (staged) das Auswertungsfenster in die
/// **eigene** Tabelle des Slice (mit Fremdschlüsseln auf die obfuskierten Zeilen) und gibt die dabei
/// **neu vergebene** [`WindowId`] zurück — die Persistenz vergibt die Identität (io:
/// `INSERT … RETURNING id`). Der Handler *staged* nur; den atomaren Commit (Ablage **und**
/// Nachrichten in **einer** Arbeitseinheit) besorgt io.
///
/// **Fehlbar und terminal** ([`WindowError`]).
pub trait AnalysisWindowRepository {
    /// Reiht das Fenster ein (Staging) und gibt die neu vergebene ID zurück, oder scheitert terminal.
    fn save(&self, window: &AnalysisWindow) -> Result<WindowId, WindowError>;
}
