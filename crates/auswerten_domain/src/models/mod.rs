//! Die eingehenden Kommandos des Slice — je Reichweite eines. Heute nur [`EvaluateWindow`], das
//! **pro-Fenster**-Kommando (ADR-0013, Entscheidung E, Reichweite 1); das fenster-übergreifende
//! kommt mit seinem eigenen Handler.

mod evaluate_window;

pub use evaluate_window::EvaluateWindow;
