//! Die DTOs und Kommandos der Adapter-Grenze (Boundary). **DTOs:** die rohe einlesen-**Zeile**
//! [`IngestedRow`] (aus dem Lese-Seam, mit ihrer „außen → innen"-Abbildung) und das obfuskierte
//! **Ausgangs-DTO** [`ObfuscatedBooking`] (Egress-Grenze, kein `Serialize`) samt seinen Token-DTOs.
//! Die eingehende Nachricht trägt nur die **ID** — die extrahiert `io` als `partition_key` der
//! Envelope und reicht sie dem [`ObfuscateCommandDecoder`], kein Wire-DTO nötig. **Kommandos:** der
//! eingehende Auslöser [`ObfuscateCommand`] samt slice-geliefertem [`ObfuscateCommandDecoder`]
//! (Envelope → Kommando, Shape Y) und das ausgehende [`ObfuscatedMessage`] (Domäne → Publish-Command,
//! ein reiner **ID-Zeiger** ohne serde — die Nutzlast liegt in der `obfuskiert`-Tabelle). Beide
//! sprechen nur `application_kernel`-Verträge; die Outbox-Mechanik lebt in `io`
//! ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)).

mod ingested_row;
mod obfuscate_command;
mod obfuscated_booking;
mod obfuscated_message;

pub use ingested_row::IngestedRow;
pub use obfuscate_command::{CONSUMED_MESSAGE_TYPE, ObfuscateCommand, ObfuscateCommandDecoder};
pub use obfuscated_booking::{AccountId, ObfuscatedBooking, ObfuscatedText, Pseudonym, Recipient};
pub use obfuscated_message::ObfuscatedMessage;
