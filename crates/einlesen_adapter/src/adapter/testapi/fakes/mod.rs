//! Die Black-Box-Test-Fläche des Slice: Fakes der öffentlichen Abstractions (Vorbild
//! `examples/Downloader/Adapter/TestApis/Fakes`). Implementieren **nie** einen internen
//! Domänen-Port direkt — die echten Port-/Publisher-Bridges laufen in `ausführen` immer über
//! sie, genau wie gegen echtes IO.

mod fake_kalender_seam;
mod fake_outbox_publisher;

pub(crate) use fake_kalender_seam::FakeKalenderSeam;
pub(crate) use fake_outbox_publisher::FakeOutboxPublisher;
