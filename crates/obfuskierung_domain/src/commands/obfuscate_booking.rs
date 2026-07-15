use crate::value_objects::BookingId;

/// Das domänen-interne Kommando des Slice: es trägt **nur die ID** der zu obfuskierenden Buchung.
/// Der Handler liest das rohe Aggregat damit aus dem einlesen-Speicher (Zeiger statt Nutzlast). Der
/// Adapter baut es aus dem Wire-Zeiger; die Domäne kennt keine Wire-/serde-Form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObfuscateBooking {
    id: BookingId,
}

impl ObfuscateBooking {
    pub fn new(id: BookingId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> &BookingId {
        &self.id
    }
}
