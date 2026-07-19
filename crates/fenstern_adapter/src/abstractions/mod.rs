//! Die öffentlichen, mechanischen Seams, die **io** erfüllt: roh-typisiert, ohne Domänen-Kenntnis.
//! Der Handler kennt sie nicht — er hängt nur an den verlangten Domänen-Ports; die Port-Bridges
//! (Modul [`crate::adapter`]) mappen dazwischen. Alle drei Richtungen sind **fehlbar und terminal**
//! (ADR-0012): [`ObfuscatedWindowSeam`] (lesen), [`AnalysisWindowStoreSeam`] (ablegen) und
//! [`WindowPublishSeam`] (publizieren). Die Seitengröße reist als Konstruktor-Wert (kein
//! Runtime-Seam) — io liest sie aus der Config und baut den Handler damit.

mod analysis_window_store_seam;
mod obfuscated_window_seam;
mod window_publish_seam;

pub use analysis_window_store_seam::{AnalysisWindowStoreSeam, StoreUnavailable};
pub use obfuscated_window_seam::{ObfuscatedWindowSeam, SourceUnavailable};
pub use window_publish_seam::{PublishUnavailable, WindowPublishSeam};
