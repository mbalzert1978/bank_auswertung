//! Die **zwei** domänen-internen Kommandos des Slice — je eines pro einzweckigem Handler
//! (ADR-0013, Entscheidung B/D): das **feldlose** [`StartAnalysis`] eröffnet den Lauf (der Anker
//! wird beschafft, nicht mitgegeben), [`AdvanceWindow`] trägt **garantiert** den Anker der Seite.
//! Der **Typ trägt die Absicht** — der illegale Zustand „leerer Anker" ist damit unkonstruierbar
//! statt geprüft (ADR-0005). Der Adapter baut sie aus der Wire-Form (ACL); die Domäne kennt keine.

mod advance_window;
mod start_analysis;

pub use advance_window::AdvanceWindow;
pub use start_analysis::StartAnalysis;
