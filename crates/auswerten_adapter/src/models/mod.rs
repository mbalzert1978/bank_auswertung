//! Die DTOs und Kommandos der Adapter-Grenze (Boundary). **DTOs:** das gelesene Fenster
//! [`AnalysisWindowRow`] samt [`WindowBookingRow`]/[`BookingDirection`] (aus dem Lese-Seam, mit
//! ihrer „außen → innen"-Abbildung und der Präsenz-Prüfung) und die abgelegte
//! [`WindowEvaluationRow`] samt [`MonthlyCashflowRow`] (Ausgangs-DTO). **Kommando:** der
//! [`EvaluateWindowCommand`] samt slice-geliefertem [`EvaluateWindowCommandDecoder`]
//! (Envelope → Kommando, Shape Y; die Fenster-ID liegt im `partition_key`).
//!
//! **Keine ausgehende Nachricht:** die pro-Fenster-Reichweite publiziert nichts — sie legt ab. Die
//! fenster-übergreifende Reichweite hängt an `fenstern`s `HistorieVollständig`, nicht an ihr.
//!
//! Alle sprechen nur `application_kernel`-Verträge; die Outbox-Mechanik lebt in `io` (ADR-0011).

mod analysis_window_row;
mod evaluate_window_command;
mod own_accounts_config;
mod window_evaluation_row;

pub use analysis_window_row::{
    AnalysisWindowRow, BookingDirection, BookingRecipient, WindowBookingRow,
};
pub use evaluate_window_command::{
    CONSUMED_MESSAGE_TYPE, EvaluateWindowCommand, EvaluateWindowCommandDecoder,
};
pub(crate) use own_accounts_config::ConfigRejected;
pub use own_accounts_config::{OwnAccountRow, OwnAccountsConfig};
pub use window_evaluation_row::{CounterAccountRow, MonthlyCashflowRow, WindowEvaluationRow};
