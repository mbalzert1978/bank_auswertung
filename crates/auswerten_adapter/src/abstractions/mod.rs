//! Die öffentlichen, **mechanischen** Abstractions des Slice — die Seams, die **io** erfüllt
//! (rusqlite/Speicher) und die in den Specs Fakes tragen.
//!
//! Die **pro-Fenster**-Reichweite braucht zwei: [`AnalysisWindowSeam`] (das eingefrorene Fenster per
//! ID lesen) und [`EvaluationStoreSeam`] (ihr Ergebnis ablegen). Die **fenster-übergreifende**
//! braucht **nur einen**, [`OccurrenceSeam`] (die Buchungen aller Fenster eines Laufs lesen) — sie
//! ist ein reines Read und legt nichts ab (ADR-0013, Entscheidung E).
//!
//! Das **Kategorisierer-Orakel** bringt zwei weitere mit: [`CategoryRegistrySeam`] (was kennt die
//! Domäne schon?) und [`CategorizerSeam`] (frag den Rest). Der zweite ist der **eine Seam dieses
//! Slice, an dem Daten die Maschine verlassen** — sein DTO trägt darum die compilerfeste
//! „kein `Serialize`"-Sperre. Sie tragen **eigene** Fehler statt sich den Beschaffungs-Fehler zu
//! teilen: ein stummes Orakel und eine unlesbare Ablage sind verschiedene Lagen.
//!
//! **Alle sind fehlbar und terminal** (ADR-0012); ihre Seam-Fehler mappen die Port-Bridges
//! „außen → innen" auf den nackten Domänen-Fehler. Den Beschaffungs-Fehler [`SourceUnavailable`]
//! teilen sich die beiden Lese-Seams: es ist derselbe Fehlschlag derselben Mechanik.

mod analysis_window_seam;
mod categorizer_seam;
mod category_registry_seam;
mod evaluation_store_seam;
mod occurrence_seam;

pub use analysis_window_seam::{AnalysisWindowSeam, SourceUnavailable};
pub use categorizer_seam::{CategorizerSeam, CategorizerUnavailable};
pub use category_registry_seam::{CategoryRegistrySeam, RegistryUnavailable};
pub use evaluation_store_seam::{EvaluationStoreSeam, StoreUnavailable};
pub use occurrence_seam::OccurrenceSeam;
