use crate::error::ObfuscationError;
use crate::value_objects::BookingId;

/// Der von der Domäne **verlangte** Publish-Port: veröffentlicht (staged) die **neu vergebene**
/// [`BookingId`] des obfuskierten Aggregats als Zeiger für den nächsten Pipeline-Schritt — nicht die
/// Nutzlast, die liegt in der `obfuskiert`-Tabelle. Der Handler *staged* nur; den Commit besorgt io.
///
/// **Fehlbar und terminal** ([`ObfuscationError`]).
pub trait BookingPublisher {
    /// Reiht die neu vergebene ID als Zeiger ein (Staging), oder scheitert terminal mit [`ObfuscationError`].
    fn publish(&self, id: &BookingId) -> Result<(), ObfuscationError>;
}
