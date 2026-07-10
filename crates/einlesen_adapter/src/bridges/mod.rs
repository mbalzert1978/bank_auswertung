//! Die Port-Bridges des Slice: sie mappen die öffentlichen, mechanischen Abstractions
//! (Modul [`crate::abstractions`], die io erfüllt) auf genau die internen Domänen-Ports, an
//! denen der [`ReadHandler`](einlesen_domain::ReadHandler) hängt — nie umgekehrt. Dazu die
//! Consume-Bridge (Wire → Handler; die Nachricht ist reiner Auslöser, den Batch beschafft
//! die [`SourceBridge`] über den Quell-Seam) und die Publisher-Bridge (Handler → Outbox).

mod calendar_bridge;
mod consume_bridge;
mod ingested_publisher;
mod money_bridge;
mod source_bridge;
mod store_bridge;

pub(crate) use calendar_bridge::CalendarBridge;
pub(crate) use consume_bridge::ConsumeBridge;
pub(crate) use ingested_publisher::IngestedPublisher;
pub(crate) use money_bridge::MoneyBridge;
pub(crate) use source_bridge::SourceBridge;
pub(crate) use store_bridge::StoreBridge;
