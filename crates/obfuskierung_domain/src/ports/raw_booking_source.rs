use crate::entities::IngestedBooking;
use crate::error::SourceError;
use crate::value_objects::BookingId;

/// Der von der Domäne **verlangte** Lese-Port, über den der Handler das rohe Aggregat einer Buchung
/// **per ID** aus dem einlesen-Speicher (Roh-Tabelle) holt. **io erfüllt ihn** (liest die Zeile und
/// deutet sie in ein [`IngestedBooking`]); in den Specs trägt ihn ein Fake.
///
/// **Fehlbar und terminal:** ein Fehlschlag ist ein redigierter [`SourceError`] — misslungene
/// Beschaffung, fehlende Zeile oder fehlendes Pflichtfeld —, nie ein überladenes `Option::None`.
pub trait RawBookingSource {
    /// Liest das rohe Aggregat zur ID, oder scheitert terminal mit [`SourceError`].
    fn read(&self, id: &BookingId) -> Result<IngestedBooking, SourceError>;
}
