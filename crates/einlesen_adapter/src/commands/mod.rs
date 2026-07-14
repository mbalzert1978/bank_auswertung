//! Die Kommandos des Slice: der **eingehende** Auslöser [`ReadCommand`] samt slice-geliefertem
//! [`ReadCommandDecoder`] (Envelope → Kommando, Shape Y) und das **ausgehende** [`IngestedMessage`]
//! (Domäne → Publish-Command, ein reiner **ID-Zeiger** ohne serde — die Nutzlast liegt im Speicher).
//! Beide sprechen nur `application_kernel`-Verträge; die Outbox-Mechanik lebt in `io`
//! ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)).

mod ingested_message;
mod read_command;

pub use ingested_message::IngestedMessage;
pub use read_command::{CONSUMED_MESSAGE_TYPE, ReadCommand, ReadCommandDecoder};
