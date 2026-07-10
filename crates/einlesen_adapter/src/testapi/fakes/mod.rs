//! Die Black-Box-Test-Fläche des Slice: Fakes der öffentlichen Abstractions (Vorbild
//! `examples/Downloader/Adapter/TestApis/Fakes`). Implementieren **nie** einen internen
//! Domänen-Port direkt — die echten Port-/Publisher-Bridges laufen in `run` immer über
//! sie, genau wie gegen echtes IO.

mod fake_calendar_seam;
mod fake_money_seam;
mod fake_outbox_publisher;
mod fake_source_seam;
mod fake_store_seam;

pub(crate) use fake_calendar_seam::FakeCalendarSeam;
pub(crate) use fake_money_seam::FakeMoneySeam;
pub(crate) use fake_outbox_publisher::FakeOutboxPublisher;
pub(crate) use fake_source_seam::FakeSourceSeam;
pub(crate) use fake_store_seam::FakeStoreSeam;
