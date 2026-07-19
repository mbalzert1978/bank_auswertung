//! Die Adapter (Port-Bridges) des Slice: sie mappen die öffentlichen, mechanischen Abstractions
//! (Modul [`crate::abstractions`], die io erfüllt) auf genau die internen Domänen-Ports, an denen
//! der domänen-interne `fenstern_domain::WindowingHandler` hängt — nie umgekehrt: [`SourceBridge`]
//! (Lese-Seam → `ObfuscatedWindowSource`, inkl. Präsenz-Prüfung), [`RepositoryBridge`]
//! (Domänen-Aggregat → `AnalysisWindowStoreSeam`) und [`WindowClosedBridge`]
//! (`WindowClosedPublisher` → `WindowPublishSeam`) und [`HistoryCompleteBridge`]
//! (`HistoryCompletePublisher` → derselbe Seam). Die Consumer-Rolle lebt seit
//! [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md) in `io`s
//! generischer `OutboxConsumer`; der [`WindowingHandler`](crate::WindowingHandler) bündelt die
//! Bridges als reine `Command → Domäne → Response`-Pipeline.

mod history_complete_bridge;
mod repository_bridge;
mod source_bridge;
mod window_closed_bridge;

pub(crate) use history_complete_bridge::HistoryCompleteBridge;
pub(crate) use repository_bridge::RepositoryBridge;
pub(crate) use source_bridge::SourceBridge;
pub(crate) use window_closed_bridge::WindowClosedBridge;
