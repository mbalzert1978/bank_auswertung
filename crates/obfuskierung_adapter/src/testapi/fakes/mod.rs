//! Die Black-Box-Test-Fläche des Slice: Fakes der öffentlichen Abstractions + des
//! `shared_kernel`-`OutboxPublisher`. Implementieren **nie** einen internen Domänen-Port direkt —
//! die echten Port-/Consume-Bridges laufen in `run` immer über sie, genau wie gegen echtes io.

mod fake_ingested_source;
mod fake_obfuscated_store;
mod fake_obfuscator;
mod fake_outbox_publisher;

pub(crate) use fake_ingested_source::FakeIngestedSource;
pub(crate) use fake_obfuscated_store::FakeObfuscatedStore;
pub(crate) use fake_obfuscator::FakeObfuscator;
pub(crate) use fake_outbox_publisher::FakeOutboxPublisher;
