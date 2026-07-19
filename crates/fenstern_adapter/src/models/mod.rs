//! Die DTOs und Kommandos der Adapter-Grenze (Boundary). **DTOs:** die obfuskierte Zeile
//! [`ObfuscatedRow`] (aus dem Lese-Seam, mit ihrer „außen → innen"-Abbildung und der
//! Präsenz-Prüfung) und die abgelegte Fenster-Zeile [`AnalysisWindowRow`] (Ausgangs-DTO, nur
//! Fremdschlüssel). **Kommandos:** der eingehende [`AdvanceWindowCommand`] samt slice-geliefertem
//! [`AdvanceWindowCommandDecoder`] (Envelope → Kommando, Shape Y; der Anker liegt als
//! stringifizierter Tag-Stamp im `partition_key`) und die ausgehende [`WindowClosedMessage`] (ein
//! reiner ID-Zeiger, kein serde) samt der [`HistoryCompleteMessage`], die den Abschluss der Kette mit
//! der Lauf-Spanne trägt. Alle sprechen nur `application_kernel`-Verträge; die
//! Outbox-Mechanik lebt in `io` (ADR-0011).

mod advance_window_command;
mod analysis_window_row;
mod history_complete_message;
mod obfuscated_row;
mod window_closed_message;

pub use advance_window_command::{
    AdvanceWindowCommand, AdvanceWindowCommandDecoder, CONSUMED_MESSAGE_TYPE,
};
pub use analysis_window_row::AnalysisWindowRow;
pub use history_complete_message::HistoryCompleteMessage;
pub use obfuscated_row::ObfuscatedRow;
pub use window_closed_message::WindowClosedMessage;
