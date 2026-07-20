//! Die **Fluent-TestApi** des Slice und ihre In-Memory-Fakes — die eine öffentliche Fläche, über die
//! die Black-Box-Specs den Slice fahren (ADR-0002). Sie arrangiert DTOs, fährt die **echte**
//! `Decoder → Handler`-Kette und gibt den **nackten** Domänen-Fehler zurück.

mod evaluation_test_api;
mod fakes;

pub use evaluation_test_api::EvaluationTestApi;
