//! Die Aggregate des Slice — über ihre **Identität** definiert, nicht über ihren Wert: das per ID
//! gelesene [`AnalysisWindow`] (die eingefrorene Eingabe) und die [`WindowEvaluation`] (das
//! abgelegte Ergebnis, dessen eigene ID die Persistenz vergibt).

mod analysis_window;
mod window_evaluation;

pub use analysis_window::AnalysisWindow;
pub use window_evaluation::WindowEvaluation;
