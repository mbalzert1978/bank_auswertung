use crate::entities::ObfuscatedBooking;
use crate::error::SourceError;
use crate::value_objects::BookingId;

/// Der von der Domäne **verlangte** Ablage-Port: legt (staged) das obfuskierte Aggregat in die
/// `obfuskiert`-Tabelle und gibt die dabei **neu vergebene** [`BookingId`] zurück — die Persistenz
/// vergibt die Identität (io: `INSERT … RETURNING id`). Der Handler *staged* nur; den atomaren Commit
/// (Ablage **und** Nachricht in **einer** Arbeitseinheit) besorgt io.
///
/// **Fehlbar und terminal** ([`SourceError`]).
pub trait ObfuscatedRepository {
    /// Reiht das obfuskierte Aggregat ein (Staging) und gibt die neu vergebene ID zurück, oder
    /// scheitert terminal mit [`SourceError`].
    fn save(&self, booking: &ObfuscatedBooking) -> Result<BookingId, SourceError>;
}
