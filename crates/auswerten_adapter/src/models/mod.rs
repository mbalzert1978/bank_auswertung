//! Die DTOs und Kommandos der Adapter-Grenze (Boundary). **DTOs:** das gelesene Fenster
//! [`AnalysisWindowRow`] samt [`WindowBookingRow`]/[`BookingDirection`] (aus dem Lese-Seam, mit
//! ihrer „außen → innen"-Abbildung und der Präsenz-Prüfung) und die abgelegte
//! [`WindowEvaluationRow`] samt [`MonthlyCashflowRow`] (Ausgangs-DTO). **Kommando:** der
//! [`EvaluateWindowCommand`] samt slice-geliefertem [`EvaluateWindowCommandDecoder`]
//! (Envelope → Kommando, Shape Y; die Fenster-ID liegt im `partition_key`).
//!
//! Die **fenster-übergreifende** Reichweite bringt ihr eigenes Paar mit: den
//! [`EvaluateHistoryCommand`] samt [`EvaluateHistoryCommandDecoder`] (die **Lauf-Spanne** liegt im
//! `partition_key`, Form `"{from}:{to}"`, wie `fenstern` sie setzt), die gelesene
//! [`RunHistoryRow`] (Eingang) und die [`HistoryEvaluationRow`] als **Ausgangs**-DTO — die
//! [`OccurrenceRow`]s der Zeitachse und die [`RecurrenceRow`]s der Wiederkehr-Erkennung (samt
//! [`RecurrenceRhythm`]/[`PriceIncreaseRow`]) und der Aderlass ([`AccountCashflowRow`],
//! [`DriverRankRow`]/[`CostDriverRow`], [`SavingsCandidateRow`]), zurückgegeben, **nicht** abgelegt
//! (kein Vorkommens-/Fold-Log). Die Spar-Schwelle kommt als [`SavingsThresholdConfig`] herein.
//!
//! Das **Kategorisierer-Orakel** bringt drei mit: die [`CategoryRequestRow`] als **Egress-DTO**
//! (hinaus — sie trägt die compilerfeste „kein `Serialize`"-Sperre), die [`CategoryAnswerRow`]
//! (herein) und die [`CategoryRow`] als Zelle des abgelegten Ergebnisses. Frage und Antwort sind
//! **je ein eigenes** schmales DTO, nie ein gemeinsames (ADR-0002).
//!
//! **Keine ausgehende Nachricht:** keine der beiden Reichweiten publiziert etwas. Die pro-Fenster
//! legt ab; die fenster-übergreifende gibt zurück und hängt an `fenstern`s `HistorieVollständig`,
//! nicht an ihr.
//!
//! Alle sprechen nur `application_kernel`-Verträge; die Outbox-Mechanik lebt in `io` (ADR-0011).

mod account_role_vocabulary;
mod aderlass_rows;
mod analysis_window_row;
mod category_exchange;
mod evaluate_history_command;
mod evaluate_window_command;
mod history_evaluation_row;
mod occurrence_row;
mod own_accounts_config;
mod run_history_row;
mod window_evaluation_row;

pub use account_role_vocabulary::account_role_label;
pub use aderlass_rows::{
    AccountCashflowRow, CostDriverRow, DriverRankRow, SavingsCandidateRow, SavingsThresholdConfig,
};
pub use analysis_window_row::{
    AnalysisWindowRow, BookingDirection, BookingRecipient, WindowBookingRow,
};
pub use category_exchange::{CategoryAnswerRow, CategoryRequestRow, CategoryRow};
pub(crate) use evaluate_history_command::RunScope;
pub use evaluate_history_command::{
    CONSUMED_HISTORY_MESSAGE_TYPE, EvaluateHistoryCommand, EvaluateHistoryCommandDecoder,
};
pub use evaluate_window_command::{
    CONSUMED_MESSAGE_TYPE, EvaluateWindowCommand, EvaluateWindowCommandDecoder,
};
pub use history_evaluation_row::{
    HistoryEvaluationRow, PriceIncreaseRow, RecurrenceRhythm, RecurrenceRow,
};
pub use occurrence_row::OccurrenceRow;
pub use own_accounts_config::{OwnAccountRow, OwnAccountsConfig};
pub use run_history_row::RunHistoryRow;
pub use window_evaluation_row::{
    CounterAccountRow, FeeRow, InternalTransferRow, MonthlyCashflowRow, WindowEvaluationRow,
};
