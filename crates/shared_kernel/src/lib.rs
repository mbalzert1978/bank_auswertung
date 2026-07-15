//! Shared-Kernel — die **`io`-private** Outbox-Mechanik der event-getriebenen Pipeline
//! ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md),
//! [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)).
//!
//! Hier liegen **nur** die Outbox-Seams, die die Infrastruktur (`io`)
//! sieht/erfüllt — technische Transport-/Ergebnistypen, **keine** Geschäftsdomäne:
//! `OutboxMessage`/`OutboxCommand`/`OutboxError`/`OutboxHandler`/`OutboxPublisher`/
//! `UnitOfWork`. Ergebnis und Fehler einer Nachrichtenverarbeitung trägt Rusts
//! `Result<(), OutboxError>`. Der Kernel läuft bewusst **nicht** auf der Domäne
//! zusammen und hat **keine** Dependencies (nur `std`).
//!
//! **Kernel-Schnitt nach dem Flip (ADR-0011, Entscheidung D):** Diese Crate ist
//! **komplett `io`-privat** — gebunden **nur** von `io`, nie von einem Slice-Adapter. Der
//! Slice↔`io`-Vertrag (`Command`/`Handler`/`Publisher`/`PublishCommand`/`CommandDecoder`)
//! lebt daneben in der **separaten Crate `application_kernel`**, die ein Slice-Adapter
//! **und** `io` binden. Rust hat kein cross-crate `internal` (C#) — Kapselung über
//! Crate-Grenzen wird über den **Crate-Schnitt** erzwungen, nicht über `pub(crate)`: weil
//! ein Adapter diese Crate nicht bindet, kann er die Outbox-Mechanik gar nicht importieren.
//! Die frühere Kante `application_kernel → shared_kernel` (der aufgelöste
//! `TypedOutboxHandler`) ist entfallen; beide Kernel sind Geschwister.
//!
//! Jeder Vertrag lebt in seiner **eigenen Datei**; dieser Root zieht die Module nur
//! zusammen und re-exportiert sie flach (`shared_kernel::OutboxHandler` usw.).
//! Vorbild: die C#-Referenz `Common/Abstractions` + `Outbox/Adapter`.

// ── Outbox-Ebene (ADR-0008) ─────────────────────────────────────────────────────
mod outbox_command;
mod outbox_error;
mod outbox_handler;
mod outbox_message;
mod outbox_publisher;
mod unit_of_work;

pub use outbox_command::OutboxCommand;
pub use outbox_error::OutboxError;
pub use outbox_handler::OutboxHandler;
pub use outbox_message::OutboxMessage;
pub use outbox_publisher::OutboxPublisher;
pub use unit_of_work::UnitOfWork;
