//! Die Black-Box-Test-Fläche des Slice: Fakes der öffentlichen Abstractions. Implementieren **nie**
//! einen internen Domänen-Port direkt — die echten Port-Bridges laufen in `run` immer über sie,
//! genau wie gegen echtes io. Jeder Fake kann zusätzlich seinen **terminalen Fehlschlag** spielen,
//! damit die Specs alle drei fehlbaren Richtungen (lesen/ablegen/publizieren) black-box treffen.

mod fake_analysis_window_store;
mod fake_obfuscated_window_source;
mod fake_window_publisher;

pub(crate) use fake_analysis_window_store::FakeAnalysisWindowStore;
pub(crate) use fake_obfuscated_window_source::FakeObfuscatedWindowSource;
pub(crate) use fake_window_publisher::FakeWindowPublisher;
