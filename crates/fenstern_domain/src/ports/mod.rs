//! Die von der Domäne **verlangten** Ports, an denen der Handler hängt: der Lese-Port
//! [`ObfuscatedWindowSource`] (die Zeilen einer Seite), das Ablage-Repository
//! [`AnalysisWindowRepository`] (vergibt die neue ID) und **zwei getrennte Sink-Ports** (ADR-0013,
//! Entscheidung B): [`WindowClosedPublisher`] (ID-Zeiger stromabwärts nach `auswerten`) und
//! [`HistoryCompletePublisher`] (Abschluss der Kette mit der Lauf-Spanne). **Alle sind fehlbar und
//! terminal** ([`WindowError`](crate::WindowError)) — ADR-0012. io erfüllt sie; in den Specs tragen
//! Fakes sie.

mod analysis_window_repository;
mod history_complete_publisher;
mod obfuscated_window_source;
mod window_closed_publisher;

pub use analysis_window_repository::AnalysisWindowRepository;
pub use history_complete_publisher::HistoryCompletePublisher;
pub use obfuscated_window_source::ObfuscatedWindowSource;
pub use window_closed_publisher::WindowClosedPublisher;
