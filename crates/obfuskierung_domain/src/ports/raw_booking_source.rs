use crate::value_objects::{BookingId, IngestedBooking};

/// Der von der Domäne **verlangte** Lese-Port, über den der Handler das rohe Aggregat einer Buchung
/// **per ID** aus dem **einlesen-Speicher** (Roh-Tabelle) holt. **io erfüllt ihn** (liest die Zeile
/// und deutet sie in ein [`IngestedBooking`] der Obfuskierungs-Domäne); die Persistenz dahinter ist
/// vertagt, ein Fake trägt ihn in den Specs. So trägt die Outbox-Nachricht nur die **ID** (Zeiger),
/// die Nutzlast liegt im Speicher.
///
/// `None`, wenn zur ID (noch) kein Aggregat vorliegt — der Adapter mappt das auf einen transienten
/// Outbox-Fehler (erneute Zustellung), denn die einlesen-Schreibung kann schlicht nachhängen.
pub trait RawBookingSource {
    /// Liest das rohe Aggregat zur ID (`None` = nicht vorhanden).
    fn read(&self, id: &BookingId) -> Option<IngestedBooking>;
}
