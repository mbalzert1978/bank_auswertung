//! Die Black-Box-Test-Fläche des Slice: Fakes der öffentlichen Abstractions + des
//! `application_kernel::Publisher`. Implementieren **nie** einen internen Domänen-Port direkt — die
//! echten Port-Bridges laufen in `run` immer über sie, genau wie gegen echtes io.

mod fake_ingested_source;
mod fake_obfuscated_store;
mod fake_obfuscator;
mod fake_publisher;

pub(crate) use fake_ingested_source::FakeIngestedSource;
pub(crate) use fake_obfuscated_store::FakeObfuscatedStore;
pub(crate) use fake_obfuscator::FakeObfuscator;
pub(crate) use fake_publisher::FakePublisher;
