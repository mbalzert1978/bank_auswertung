//! Die öffentlichen, **mechanischen** Abstractions des Slice — die Seams, die **io** erfüllt
//! (sqlx/Speicher) und die in den Specs Fakes tragen.
//!
//! Die **pro-Fenster**-Reichweite braucht zwei: [`AnalysisWindowSeam`] (das eingefrorene Fenster per
//! ID lesen) und [`EvaluationStoreSeam`] (ihr Ergebnis ablegen). Die **fenster-übergreifende**
//! braucht **nur einen**, [`OccurrenceSeam`] (die Buchungen aller Fenster eines Laufs lesen) — sie
//! ist ein reines Read und legt nichts ab (ADR-0013, Entscheidung E).
//!
//! **Alle sind fehlbar und terminal** (ADR-0012); ihre Seam-Fehler mappen die Port-Bridges
//! „außen → innen" auf den nackten Domänen-Fehler. Den Beschaffungs-Fehler [`SourceUnavailable`]
//! teilen sich die beiden Lese-Seams: es ist derselbe Fehlschlag derselben Mechanik.

mod analysis_window_seam;
mod evaluation_store_seam;
mod occurrence_seam;

pub use analysis_window_seam::{AnalysisWindowSeam, SourceUnavailable};
pub use evaluation_store_seam::{EvaluationStoreSeam, StoreUnavailable};
pub use occurrence_seam::OccurrenceSeam;
