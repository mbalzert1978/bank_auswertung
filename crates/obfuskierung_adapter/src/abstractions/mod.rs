//! Die öffentlichen, mechanischen Seams, die **io** erfüllt: roh-typisiert, ohne Domänen-Kenntnis.
//! Der Handler kennt sie nicht — er hängt nur an den verlangten Domänen-Ports; die Port-Bridges
//! (Modul [`crate::adapter`]) mappen dazwischen. Der Seed reist als Konstruktor-Wert (kein
//! Runtime-Seam) — io liest ihn aus der Config und baut die Bridges damit.

mod ingested_source_seam;
mod obfuscated_store_seam;
mod obfuscator_seam;

pub use ingested_source_seam::IngestedSourceSeam;
pub use obfuscated_store_seam::{ObfuscatedStoreSeam, StoreUnavailable};
pub use obfuscator_seam::ObfuscatorSeam;
