use crate::error::WindowError;
use crate::value_objects::WindowId;

/// Der von der Domäne **verlangte** Sink-Port **stromabwärts nach `auswerten`**: veröffentlicht
/// (staged) die **neu vergebene** [`WindowId`] des abgelegten Fensters als Zeiger — nicht die
/// Nutzlast, die liegt in der Fenster-Tabelle (Tabelle = Wahrheit, Message = Pointer).
///
/// **Fehlbar und terminal** ([`WindowError`]).
pub trait WindowClosedPublisher {
    /// Reiht die neu vergebene Fenster-ID als Zeiger ein (Staging), oder scheitert terminal.
    fn publish(&self, id: &WindowId) -> Result<(), WindowError>;
}
