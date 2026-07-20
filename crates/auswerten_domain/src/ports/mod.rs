//! Die Ports, die die **Domäne verlangt** — nach innen zeigende Schnittstellen, die der Adapter
//! über Port-Bridges auf die öffentlichen Seams (io/Fakes) mappt.
//!
//! Die **pro-Fenster**-Reichweite verlangt zwei: [`AnalysisWindowSource`] liest das eingefrorene
//! Fenster per ID, [`EvaluationRepository`] legt ihr Ergebnis ab. Die **fenster-übergreifende**
//! verlangt **nur einen**, [`OccurrenceSource`] — sie ist ein reines **Read** und hat bewusst kein
//! Schreib-Gegenstück (kein Vorkommens-/Fold-Log, ADR-0013 Entscheidung E).
//!
//! Die pro-Fenster-Reichweite verlangt zwei weitere für das **Kategorisierer-Orakel** (ADR-0004):
//! [`CategoryRegistry`] beantwortet, was die Domäne schon **kennt**, und [`Categorizer`] fragt
//! allein den Rest. Beide sind Ports der **Domäne**, nicht Schritte der Pipeline: dass gefragt
//! wird, entscheidet die Analyse.
//!
//! **Alle sind fehlbar und terminal** (ADR-0012).

mod analysis_window_source;
mod categorizer;
mod category_registry;
mod evaluation_repository;
mod occurrence_source;

pub use analysis_window_source::AnalysisWindowSource;
pub use categorizer::Categorizer;
pub use category_registry::CategoryRegistry;
pub use evaluation_repository::EvaluationRepository;
pub use occurrence_source::OccurrenceSource;
