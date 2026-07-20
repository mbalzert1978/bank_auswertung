//! Die Ports, die die **Domäne verlangt** — nach innen zeigende Schnittstellen, die der Adapter
//! über Port-Bridges auf die öffentlichen Seams (io/Fakes) mappt: [`AnalysisWindowSource`] liest das
//! eingefrorene Fenster per ID, [`EvaluationRepository`] legt das Pro-Fenster-Ergebnis ab. **Beide
//! sind fehlbar und terminal** (ADR-0012).

mod analysis_window_source;
mod evaluation_repository;

pub use analysis_window_source::AnalysisWindowSource;
pub use evaluation_repository::EvaluationRepository;
