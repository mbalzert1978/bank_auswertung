//! Die Aggregate des Slice — über ihre **Identität** definiert, nicht über ihren Wert: das per ID
//! gelesene [`AnalysisWindow`] (die eingefrorene Eingabe) und die [`WindowEvaluation`] (das
//! abgelegte Ergebnis, dessen eigene ID die Persistenz vergibt).
//!
//! Die **fenster-übergreifende** Reichweite bringt ihr Paar mit: die [`RunHistory`] (die gelesene
//! Eingabe über alle Fenster des Laufs) und die [`HistoryEvaluation`] (das **zurückgegebene**, nie
//! abgelegte Ergebnis — Vorkommen und Wiederkehr).

mod analysis_window;
mod history_evaluation;
mod run_history;
mod window_evaluation;

pub use analysis_window::AnalysisWindow;
pub use history_evaluation::HistoryEvaluation;
pub use run_history::RunHistory;
pub use window_evaluation::WindowEvaluation;
