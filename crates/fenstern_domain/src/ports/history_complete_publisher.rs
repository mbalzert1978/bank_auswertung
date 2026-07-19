use crate::error::WindowError;
use crate::value_objects::WindowSpan;

/// Der von der Domäne **verlangte** Sink-Port für den **Abschluss der Kette**: veröffentlicht
/// (staged) die **Lauf-Spanne**, sobald die Historie erschöpft ist (ADR-0013, Entscheidung B).
///
/// Er ist der Auslöser für die **fenster-übergreifende** Reichweite von `auswerten` (Entscheidung E):
/// die Wiederkehr-Erkennung muss **einmal, bei vollständiger Historie** laufen, und nur `fenstern`
/// weiß, wann das der Fall ist — der Pager läuft neu→alt, `auswerten` kann die letzte Seite nicht
/// selbst erkennen. Die mitgetragene Spanne scopt die Query auf genau diesen Lauf.
///
/// **Fehlbar und terminal** ([`WindowError`]).
pub trait HistoryCompletePublisher {
    /// Reiht den Abschluss mit der Lauf-Spanne ein (Staging), oder scheitert terminal.
    fn publish(&self, span: &WindowSpan) -> Result<(), WindowError>;
}
