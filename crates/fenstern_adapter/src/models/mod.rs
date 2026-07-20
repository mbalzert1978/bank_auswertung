//! Die DTOs und Kommandos der Adapter-Grenze (Boundary). **DTOs:** die obfuskierte Zeile
//! [`ObfuscatedRow`] (aus dem Lese-Seam, mit ihrer „außen → innen"-Abbildung und der
//! Präsenz-Prüfung) und die abgelegte Fenster-Zeile [`AnalysisWindowRow`] (Ausgangs-DTO, nur
//! Fremdschlüssel). **Kommandos** — **zwei** eingehende, je eines pro einzweckigem Handler: der
//! **feldlose** [`StartAnalysisCommand`] (der Anstoß, ein reiner Trigger; sein
//! [`StartAnalysisCommandDecoder`] hat nichts zu deuten, weil kein Anker über die Wire reist) und
//! der [`AdvanceWindowCommand`] samt slice-geliefertem
//! [`AdvanceWindowCommandDecoder`] (Envelope → Kommando, Shape Y; der Anker liegt als
//! stringifizierter Tag-Stamp im `partition_key`). Dazu die ausgehende [`WindowClosedMessage`] (ein
//! reiner ID-Zeiger, kein serde), die [`AdvanceWindowMessage`] der Selbst-Fortsetzung (derselbe
//! Nachrichtentyp, den der Slice konsumiert) und die [`HistoryCompleteMessage`], die den Abschluss
//! der Kette mit der Lauf-Spanne trägt. Alle sprechen nur `application_kernel`-Verträge; die
//! Outbox-Mechanik lebt in `io` (ADR-0011).

mod advance_window_command;
mod advance_window_message;
mod analysis_window_row;
mod history_complete_message;
mod obfuscated_row;
mod start_analysis_command;
mod window_closed_message;

pub use advance_window_command::{
    AdvanceWindowCommand, AdvanceWindowCommandDecoder, CONSUMED_MESSAGE_TYPE,
};
pub use advance_window_message::AdvanceWindowMessage;
pub use analysis_window_row::AnalysisWindowRow;
pub use history_complete_message::HistoryCompleteMessage;
pub use obfuscated_row::ObfuscatedRow;
pub use start_analysis_command::{
    START_MESSAGE_TYPE, StartAnalysisCommand, StartAnalysisCommandDecoder,
};
pub use window_closed_message::WindowClosedMessage;
