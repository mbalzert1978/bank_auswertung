//! Das domänen-interne Kommando des Slice: [`AdvanceWindow`] trägt den **Anker** der Seite. Der
//! Adapter baut es aus dem Wire-Zeiger (ACL); die Domäne kennt keine Wire-Form.

mod advance_window;

pub use advance_window::AdvanceWindow;
