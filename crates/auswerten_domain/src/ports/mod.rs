//! Die Ports, die die **Domäne verlangt** — nach innen zeigende Schnittstellen, die der Adapter
//! über Port-Bridges auf die öffentlichen Seams (io/Fakes) mappt.
//!
//! Die **pro-Fenster**-Reichweite verlangt zwei: [`AnalysisWindowSource`] liest das eingefrorene
//! Fenster per ID, [`EvaluationRepository`] legt ihr Ergebnis ab. Die **fenster-übergreifende**
//! verlangt **nur einen**, [`OccurrenceSource`] — sie ist ein reines **Read** und hat bewusst kein
//! Schreib-Gegenstück (kein Vorkommens-/Fold-Log, ADR-0013 Entscheidung E).
//!
//! **Alle sind fehlbar und terminal** (ADR-0012).

mod analysis_window_source;
mod evaluation_repository;
mod occurrence_source;

pub use analysis_window_source::AnalysisWindowSource;
pub use evaluation_repository::EvaluationRepository;
pub use occurrence_source::OccurrenceSource;
