use crate::value_objects::BookingId;

/// Der von der Domäne **verlangte** Port, um die **ID** des obfuskierten Aggregats als Zeiger für
/// den nächsten Pipeline-Schritt zu **publizieren** (nicht die Nutzlast — die liegt im Repository).
/// Wie der Einlese-`EventSink` *staged* der Handler nur; der Commit (Zeiger **und** Ablage atomar)
/// gehört io. Der Port spricht **Domänen-Vokabular** ([`BookingId`]); die Abbildung auf das
/// Outbox-Wire-DTO ist Adapter-Sache.
pub trait BookingPublisher {
    /// Reiht die ID als `Obfuscated`-Zeiger in die offene Arbeitseinheit ein (Staging).
    fn publish(&self, id: &BookingId);
}
