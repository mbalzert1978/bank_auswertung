//! Application-Kernel — der **`io`-sichtbare Slice↔`io`-Vertrag** des modularen Monolithen
//! ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md),
//! [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)).
//!
//! **Kernel-Schnitt nach dem Flip (ADR-0011, Entscheidung D):** Diese Crate ist der
//! Vertrag, über den ein Slice-Adapter und `io` sich treffen — wie ein Slice seinen Handler,
//! sein Kommando und seinen Ausgang strukturiert (`Command`/`Handler`/`Publisher`/
//! `PublishCommand`) und wie `io` die Envelope an ihn übergibt (`CommandDecoder`, Shape Y).
//! Gebunden von **`<slice>_adapter` und `io`**. Die **io-private Outbox-Mechanik**
//! (`OutboxMessage`/`OutboxError`/`OutboxHandler`/…) liegt `io`-intern daneben — sie ist aus
//! keinem Slice-Adapter erreichbar, so kann ein Adapter die Outbox-Mechanik gar nicht
//! importieren.
//!
//! **Dependency-frei:** Seit `TypedOutboxHandler` sich aufgelöst hat (die Verdrahtung
//! Envelope → Request → `Result` lebt jetzt einmal in `io`s generischem Consumer), hängt
//! diese Crate an **nichts** mehr. Sie ist der **einzige** Kernel — die frühere
//! Schwester-Crate mit den Outbox-Seams wurde in `io` aufgelöst (Issue 0032).
//!
//! Jeder Vertrag lebt in seiner eigenen Datei; dieser Root zieht die Module zusammen und
//! re-exportiert sie flach (`application_kernel::Handler` usw.).

mod command;
mod command_decoder;
mod handler;
mod publish_command;
mod publisher;

pub use command::Command;
pub use command_decoder::CommandDecoder;
pub use handler::Handler;
pub use publish_command::PublishCommand;
pub use publisher::Publisher;
