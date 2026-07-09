//! Die Bridges des Slice: die Port-Bridges [`SourceBridge`] (Lese-Seam → `RawBookingSource`),
//! [`ObfuscatorBridge`] (`ObfuscatorSeam` → `Obfuscator`), [`RepositoryBridge`] (Domänen-Aggregat →
//! `ObfuscatedStoreSeam`) und [`PublisherBridge`] (`BookingPublisher` → `OutboxPublisher`) sowie die
//! [`ConsumeBridge`] (Wire-Zeiger → Kommando → Handler).

mod consume_bridge;
mod obfuscator_bridge;
mod publisher_bridge;
mod repository_bridge;
mod source_bridge;

pub(crate) use consume_bridge::{CONSUMED_MESSAGE_TYPE, ConsumeBridge};
pub(crate) use obfuscator_bridge::ObfuscatorBridge;
pub(crate) use publisher_bridge::PublisherBridge;
pub(crate) use repository_bridge::RepositoryBridge;
pub(crate) use source_bridge::SourceBridge;
