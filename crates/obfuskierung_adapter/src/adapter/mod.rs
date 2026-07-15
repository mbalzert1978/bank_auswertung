//! Die Adapter (Port-Bridges) des Slice: sie mappen die öffentlichen, mechanischen Abstractions (Modul
//! [`crate::abstractions`], die io erfüllt) auf genau die internen Domänen-Ports, an denen der
//! domänen-interne `obfuskierung_domain::ObfuscationHandler` hängt — nie umgekehrt: [`SourceBridge`]
//! (Lese-Seam → `RawBookingSource`), [`ObfuscatorBridge`] (`ObfuscatorSeam` → `Obfuscator`),
//! [`RepositoryBridge`] (Domänen-Aggregat → `ObfuscatedStoreSeam`) und [`PublisherBridge`]
//! (`BookingPublisher` → `application_kernel::Publisher`). Die frühere Consume-Bridge ist entfallen:
//! die Consumer-Rolle lebt seit [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)
//! in `io`s generischer `OutboxConsumer`; der [`ObfuscatingHandler`](crate::ObfuscatingHandler)
//! bündelt die Bridges als reine `Command → Domäne → Response`-Pipeline.

mod obfuscator_bridge;
mod publisher_bridge;
mod repository_bridge;
mod source_bridge;

pub(crate) use obfuscator_bridge::ObfuscatorBridge;
pub(crate) use publisher_bridge::PublisherBridge;
pub(crate) use repository_bridge::RepositoryBridge;
pub(crate) use source_bridge::SourceBridge;
