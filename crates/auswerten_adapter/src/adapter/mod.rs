//! Die **Port-Bridges**: sie erfüllen die von der Domäne verlangten Ports und mappen sie auf die
//! öffentlichen, mechanischen Abstractions (Modul [`abstractions`](crate::abstractions)), die io
//! erfüllt. Hier — und nur hier — laufen die beiden Richtungen der ACL zusammen: „außen → innen"
//! (rohe Zeile → Aggregat, Seam-Fehler → terminaler Domänen-Fehler) und „innen → außen" (Aggregat →
//! Zeile).

mod categorizer_bridge;
mod occurrence_bridge;
mod registry_bridge;
mod repository_bridge;
mod source_bridge;

pub(crate) use categorizer_bridge::CategorizerBridge;
pub(crate) use occurrence_bridge::OccurrenceBridge;
pub(crate) use registry_bridge::RegistryBridge;
pub(crate) use repository_bridge::RepositoryBridge;
pub(crate) use source_bridge::SourceBridge;
