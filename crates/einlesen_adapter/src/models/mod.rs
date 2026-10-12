//! Die DTOs und Kommandos der Adapter-Grenze (Boundary). **DTOs:** das eingehende Seam-DTO
//! [`RawRecord`] (außen → innen, über den Quell-Seam beschafft) und das ausgehende [`Line`] samt
//! adapter-eigener [`Direction`] (innen → außen) — beide tragen nur primitive/adapter-eigene Typen,
//! kein Domänentyp überquert die Adapter-Grenze. **Kommandos:** der eingehende Auslöser
//! [`ReadCommand`] samt slice-geliefertem [`ReadCommandDecoder`] (Envelope → Kommando, Shape Y) und
//! das ausgehende [`IngestedMessage`] (Domäne → Publish-Command, ein reiner **ID-Zeiger** ohne
//! serde — die Nutzlast liegt im Speicher). Die Kommandos sprechen nur `application_kernel`-Verträge;
//! die Outbox-Mechanik lebt in `io`
//! ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)).

mod ingested_message;
mod line;
mod raw_record;
mod read_command;

pub use ingested_message::IngestedMessage;
pub use line::{Direction, Line};
pub use raw_record::{RawRecord, RecordStatus};
pub use read_command::{CONSUMED_MESSAGE_TYPE, ReadCommand, ReadCommandDecoder};
