//! Die Kommandos des Slice: der **eingehende** Auslöser [`ObfuscateCommand`] samt slice-geliefertem
//! [`ObfuscateCommandDecoder`] (Envelope → Kommando, Shape Y) und das **ausgehende**
//! [`ObfuscatedMessage`] (Domäne → Publish-Command, ein reiner **ID-Zeiger** ohne serde — die Nutzlast
//! liegt in der `obfuskiert`-Tabelle). Beide sprechen nur `application_kernel`-Verträge; die
//! Outbox-Mechanik lebt in `io`
//! ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)).

mod obfuscate_command;
mod obfuscated_message;

pub use obfuscate_command::{CONSUMED_MESSAGE_TYPE, ObfuscateCommand, ObfuscateCommandDecoder};
pub use obfuscated_message::ObfuscatedMessage;
