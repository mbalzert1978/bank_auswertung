//! Die von der Domäne **verlangten** Ports, an denen der Handler hängt: der Lese-Port
//! [`ObfuscatedWindowSource`] (die Zeilen einer Seite), der Anker-Port [`WindowAnchor`]
//! (`max(booking_day < from)` — Fortsetzung **und** Terminierung aus einer Query-Art), das
//! Ablage-Repository [`AnalysisWindowRepository`] (vergibt die neue ID) und **drei** Sink-Ports
//! (ADR-0013, Entscheidung B): [`WindowClosedPublisher`] (ID-Zeiger stromabwärts nach `auswerten`)
//! sowie die beiden **einander ausschließenden** Fortsetzungs-Zweige [`AdvanceWindowPublisher`]
//! (die Kette geht weiter) und [`HistoryCompletePublisher`] (die Historie ist erschöpft).
//! **Alle sind fehlbar und terminal** ([`WindowError`](crate::WindowError)) — ADR-0012. io erfüllt
//! sie; in den Specs tragen Fakes sie.

mod advance_window_publisher;
mod analysis_window_repository;
mod history_complete_publisher;
mod obfuscated_window_source;
mod window_anchor;
mod window_closed_publisher;

pub use advance_window_publisher::AdvanceWindowPublisher;
pub use analysis_window_repository::AnalysisWindowRepository;
pub use history_complete_publisher::HistoryCompletePublisher;
pub use obfuscated_window_source::ObfuscatedWindowSource;
pub use window_anchor::WindowAnchor;
pub use window_closed_publisher::WindowClosedPublisher;
