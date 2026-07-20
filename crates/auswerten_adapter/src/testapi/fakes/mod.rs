//! Die **In-Memory-Doubles** der Seams, die die Fluent-TestApi verdrahtet: die Fenster-Ablage von
//! `fenstern` ([`FakeAnalysisWindowSource`]) und die eigene Ergebnis-Tabelle
//! ([`FakeEvaluationStore`]). Sie spielen das vertagte `io` — beide auch in ihrer **terminalen**
//! Fehlvariante (ADR-0012).

mod fake_analysis_window_source;
mod fake_evaluation_store;

pub(crate) use fake_analysis_window_source::FakeAnalysisWindowSource;
pub(crate) use fake_evaluation_store::FakeEvaluationStore;
