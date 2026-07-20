//! Die **Fluent-TestApis** des Slice und ihre In-Memory-Fakes — die eine öffentliche Fläche, über
//! die die Black-Box-Specs den Slice fahren (ADR-0002). Sie arrangieren DTOs, fahren die **echte**
//! `Decoder → Handler`-Kette und geben den **nackten** Domänen-Fehler zurück.
//!
//! **Eine je Reichweite** (ADR-0013, Entscheidung E), weil die beiden Handler verschiedene Lagen
//! herstellen: die [`EvaluationTestApi`] arrangiert **ein** Fenster samt Eigene-Konten-Config und
//! prüft die **Ablage**; die [`HistoryTestApi`] arrangiert **mehrere** Fenster eines Laufs und prüft
//! den **Rückgabewert** — sie legt nichts ab. Eine gemeinsame TestApi müsste beide Lagen tragen und
//! verwischte damit genau die Einzweckigkeit, die die zwei Handler ausmacht.

mod evaluation_test_api;
mod fakes;
mod history_test_api;

pub use evaluation_test_api::EvaluationTestApi;
pub use history_test_api::HistoryTestApi;
