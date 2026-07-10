use crate::value_objects::{BookingId, IngestedBooking};

/// Der von der Domäne **verlangte** Lese-Port, über den der Handler das rohe Aggregat einer Buchung
/// **per ID** aus dem **einlesen-Speicher** (Roh-Tabelle) holt. **io erfüllt ihn** (liest die Zeile
/// und deutet sie in ein [`IngestedBooking`] der Obfuskierungs-Domäne); die Persistenz dahinter ist
/// vertagt, ein Fake trägt ihn in den Specs. So trägt die Outbox-Nachricht nur die **ID** (Zeiger),
/// die Nutzlast liegt im Speicher.
pub trait RawBookingSource {
    /// Liest das rohe Aggregat zur ID — **dreiwertig** ([`BookingLookup`]): vorhanden, (noch)
    /// abwesend (transient) oder vorhanden-aber-ungültig (terminal, redigierter Feldname).
    fn read(&self, id: &BookingId) -> BookingLookup;
}

/// Das **dreiwertige** Ergebnis eines Lese-Versuchs — die drei Ausgänge liegen im **Typ**, nicht in
/// einem überladenen `Option::None`, damit der Handler „abwesend" (transient) sauber von „ungültig"
/// (terminal) trennen kann.
pub enum BookingLookup {
    /// Zur ID liegt ein deutbares rohes Aggregat vor. **Geboxt**, weil das Aggregat weit größer ist
    /// als die schlanken Fehler-Varianten (`clippy::large_enum_variant`).
    Found(Box<IngestedBooking>),
    /// Zur ID liegt (noch) **keine** Zeile vor — **transient** (die einlesen-Schreibung kann
    /// nachhängen); der Adapter mappt das auf erneute Zustellung (Retry).
    Absent,
    /// Zur ID liegt eine Zeile vor, aber ein **Pflichtfeld fehlt/ist leer** — **terminal**
    /// (Dead-Letter). Trägt **nur** den statischen Feldnamen (redigiert, nie ein Rohwert).
    Malformed { field: &'static str },
}
