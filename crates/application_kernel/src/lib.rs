//! Application-Kernel — die **slice-internen** Handler-Bau-Konventionen des
//! modularen Monolithen. Anders als der `shared_kernel` (die io-facing
//! Outbox-Seams) sehen diese Verträge **nur** die Slices selbst (ihre Domäne über
//! den Adapter), **nie** das `io`-Crate: wie ein Slice seinen Handler und seine
//! Kommandos strukturiert, geht die Infrastruktur nichts an
//! ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)).
//!
//! **Warum eine eigene Crate statt `pub(crate)` im `shared_kernel`?** Rust kennt
//! keine assembly-weite `internal`-Sichtbarkeit wie C#; über Crate-Grenzen ist
//! alles entweder unsichtbar oder `pub`. Kapselung drückt man deshalb durch den
//! **Crate-Schnitt** aus: was `io` nicht sehen soll, lebt in einer Crate, die `io`
//! nicht bindet. Diese Crate hängt ihrerseits an `shared_kernel` (die
//! [`TypedOutboxHandler`]-Basis erzeugt einen `shared_kernel::OutboxHandler`) —
//! aber an *ihr* hängen nur die `<slice>_adapter`-Crates (vom `architecture`-Test
//! erzwungen).
//!
//! Enthält die **Handler-Ebene** (`Command`/`PublishCommand`/`Handler`/`Publisher`)
//! und die Consume-Bridge-Basis `TypedOutboxHandler`. Dependency: nur
//! `shared_kernel`. Jeder Vertrag lebt in seiner eigenen Datei; dieser Root zieht
//! die Module zusammen und re-exportiert sie flach (`application_kernel::Handler`
//! usw.).

mod command;
mod handler;
mod publish_command;
mod publisher;
mod typed_outbox_handler;

pub use command::Command;
pub use handler::Handler;
pub use publish_command::PublishCommand;
pub use publisher::Publisher;
pub use typed_outbox_handler::TypedOutboxHandler;
