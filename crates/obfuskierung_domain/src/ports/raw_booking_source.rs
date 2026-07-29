use crate::entities::IngestedBooking;
use crate::error::ObfuscationError;
use crate::value_objects::BookingId;

/// Der von der Domäne **verlangte** Lese-Port, über den der Handler das rohe Aggregat einer Buchung
/// **per ID** aus dem einlesen-Speicher (Roh-Tabelle) holt. **io erfüllt ihn** (liest die Zeile und
/// deutet sie in ein [`IngestedBooking`]); in den Specs trägt ihn ein Fake.
///
/// **Fehlbar und terminal:** ein Fehlschlag ist ein redigierter [`ObfuscationError`] — misslungene
/// Beschaffung, fehlende Zeile oder fehlendes Pflichtfeld —, nie ein überladenes `Option::None`.
pub trait RawBookingSource {
    /// Liest das rohe Aggregat zur ID, oder scheitert terminal mit [`ObfuscationError`].
    fn read(&self, id: &BookingId) -> Result<IngestedBooking, ObfuscationError>;
}
