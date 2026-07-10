//! Die DTOs der Adapter-Grenze: die rohe einlesen-**Zeile** [`IngestedRow`] (aus dem Lese-Seam, mit
//! ihrer „außen → innen"-Abbildung) und das obfuskierte **Ausgangs-DTO** [`ObfuscatedBooking`]
//! (Egress-Grenze, kein `Serialize`) samt seinen Token-DTOs. Die eingehende Nachricht trägt nur die
//! **ID** — die liest die Consume-Bridge aus der `OutboxMessage`-Hülle, kein Wire-DTO nötig.

mod ingested_row;
mod obfuscated_booking;

pub use ingested_row::IngestedRow;
pub use obfuscated_booking::{AccountId, ObfuscatedBooking, ObfuscatedText, Pseudonym, Recipient};
