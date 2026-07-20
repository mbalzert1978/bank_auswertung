//! Die **Port-Bridges**: sie erfüllen die von der Domäne verlangten Ports und mappen sie auf die
//! öffentlichen, mechanischen Abstractions (Modul [`abstractions`](crate::abstractions)), die io
//! erfüllt. Hier — und nur hier — laufen die beiden Richtungen der ACL zusammen: „außen → innen"
//! (rohe Zeile → Aggregat, Seam-Fehler → terminaler Domänen-Fehler) und „innen → außen" (Aggregat →
//! Zeile).

mod occurrence_bridge;
mod repository_bridge;
mod source_bridge;

pub(crate) use occurrence_bridge::OccurrenceBridge;
pub(crate) use repository_bridge::RepositoryBridge;
pub(crate) use source_bridge::SourceBridge;
