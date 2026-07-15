//! Die Adapter (Port-Bridges) des Slice: sie mappen die öffentlichen, mechanischen Abstractions
//! (Modul [`crate::abstractions`], die io erfüllt) auf genau die internen Domänen-Ports, an
//! denen der interne `einlesen_domain::ReadHandler` hängt — nie umgekehrt. Dazu die
//! Publisher-Bridge (`EventSink` → Publish-Seam) und die Quell-Bridge
//! (Beschaffung + Präsenz-Prüfung hinter dem Quell-Seam). Die frühere Consume-Bridge ist
//! entfallen: die Consumer-Rolle lebt seit [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)
//! in `io`s generischer `OutboxConsumer`; der [`ReadingHandler`](crate::ReadingHandler)
//! bündelt die Bridges als reine `Command → Domäne → Response`-Pipeline.

mod calendar_bridge;
mod ingested_publisher;
mod money_bridge;
mod source_bridge;
mod store_bridge;

pub(crate) use calendar_bridge::CalendarBridge;
pub(crate) use ingested_publisher::IngestedPublisher;
pub(crate) use money_bridge::MoneyBridge;
pub(crate) use source_bridge::SourceBridge;
pub(crate) use store_bridge::StoreBridge;
