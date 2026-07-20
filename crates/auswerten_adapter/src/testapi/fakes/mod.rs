//! Die **In-Memory-Doubles** der Seams, die die Fluent-TestApis verdrahten: die Fenster-Ablage von
//! `fenstern` ([`FakeAnalysisWindowSource`]), die eigene Ergebnis-Tabelle
//! ([`FakeEvaluationStore`]) und die fenster-übergreifende Sicht auf die Fenster-Ablage
//! ([`FakeOccurrenceSource`], der die Lauf-Spanne als Scope filtert — die `WHERE`-Klausel, die io
//! später formuliert). Dazu die beiden Hälften des Kategorisierer-Orakels: das
//! [`FakeCategoryRegistry`] (was kennt die Domäne schon?) und der [`FakeCategorizer`], der
//! festhält, **wonach** gefragt wurde. Sie spielen das vertagte `io` — jeder auch in seiner
//! **terminalen** Fehlvariante (ADR-0012).

mod fake_analysis_window_source;
mod fake_categorizer;
mod fake_category_registry;
mod fake_evaluation_store;
mod fake_occurrence_source;

pub(crate) use fake_analysis_window_source::FakeAnalysisWindowSource;
pub(crate) use fake_categorizer::FakeCategorizer;
pub(crate) use fake_category_registry::FakeCategoryRegistry;
pub(crate) use fake_evaluation_store::FakeEvaluationStore;
pub(crate) use fake_occurrence_source::FakeOccurrenceSource;
