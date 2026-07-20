//! Die eingehenden Kommandos des Slice — **je Reichweite eines** (ADR-0013, Entscheidung E):
//! [`EvaluateWindow`] für die **pro-Fenster**-Reichweite (aus `FensterGeschlossen`) und
//! [`EvaluateHistory`] für die **fenster-übergreifende** (aus `HistorieVollständig`, samt der
//! Lauf-Spanne, die sie scopt).
//!
//! **Zwei Typen statt eines mit Flag** (ADR-0005): so ist es unkonstruierbar, die eine Reichweite
//! aus dem Auslöser der anderen anzustoßen.

mod evaluate_history;
mod evaluate_window;

pub use evaluate_history::EvaluateHistory;
pub use evaluate_window::EvaluateWindow;
