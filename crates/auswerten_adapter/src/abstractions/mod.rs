//! Die öffentlichen, **mechanischen** Abstractions des Slice — die Seams, die **io** erfüllt
//! (sqlx/Speicher) und die in den Specs Fakes tragen: [`AnalysisWindowSeam`] (das eingefrorene
//! Fenster per ID lesen) und [`EvaluationStoreSeam`] (das Pro-Fenster-Ergebnis ablegen). **Beide
//! sind fehlbar und terminal** (ADR-0012); ihre Seam-Fehler mappen die Port-Bridges „außen → innen"
//! auf den nackten Domänen-Fehler.

mod analysis_window_seam;
mod evaluation_store_seam;

pub use analysis_window_seam::{AnalysisWindowSeam, SourceUnavailable};
pub use evaluation_store_seam::{EvaluationStoreSeam, StoreUnavailable};
