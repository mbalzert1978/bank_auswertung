//! Die öffentlichen, mechanischen Seams, die **io** erfüllt: roh-typisiert, ohne Domänen-Kenntnis.
//! Der Handler kennt sie nicht — er hängt nur an den verlangten Domänen-Ports; die Port-Bridges
//! (Modul [`crate::adapter`]) mappen dazwischen. Der Seed reist als Konstruktor-Wert (kein
//! Runtime-Seam) — io liest ihn aus der Config und baut die Bridges damit.
//!
//! **Die beiden I/O-Seams sind fehlbar und terminal** (ADR-0012); ihre Seam-Fehler mappen die
//! Port-Bridges „außen → innen" auf den nackten Domänen-Fehler. Sie tragen **eigene** Fehler statt
//! sich einen zu teilen: der Beschaffungs-Fehler [`SourceUnavailable`] hält den mißlungenen Zugriff
//! auf den einlesen-Bestand von der gültigen Auskunft „zu dieser ID liegt nichts vor" (`Ok(None)`)
//! auseinander, [`StoreUnavailable`] meldet die unerreichbare Ablage — eine unlesbare Quelle und
//! eine unschreibbare Tabelle sind verschiedene Lagen. Der [`ObfuscatorSeam`] hat bewusst **keinen**
//! Fehlerkanal: er ist eine reine, in-process Berechnung.

mod ingested_source_seam;
mod obfuscated_store_seam;
mod obfuscator_seam;

pub use ingested_source_seam::{IngestedSourceSeam, SourceUnavailable};
pub use obfuscated_store_seam::{ObfuscatedStoreSeam, StoreUnavailable};
pub use obfuscator_seam::ObfuscatorSeam;
