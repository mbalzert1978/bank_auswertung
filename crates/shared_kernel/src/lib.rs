//! Shared-Kernel — die **io-facing** Verträge der event-getriebenen Pipeline
//! ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)).
//!
//! Hier liegen **nur** die Outbox-Seams, die die Infrastruktur (`io`)
//! sieht/erfüllt — technische Transport-/Ergebnistypen, **keine** Geschäftsdomäne:
//! `OutboxMessage`/`OutboxCommand`/`OutboxError`/`OutboxHandler`/`OutboxPublisher`/
//! `UnitOfWork`. Ergebnis und Fehler einer Nachrichtenverarbeitung trägt Rusts
//! `Result<(), OutboxError>`. Der Kernel läuft bewusst **nicht** auf der Domäne
//! zusammen und hat **keine** Dependencies (nur `std`).
//!
//! **Bewusst NICHT hier:** die slice-internen Handler-Bau-Konventionen
//! (`Command`/`Handler`/`Publisher`/`TypedOutboxHandler`) leben in einer
//! **separaten Crate `application_kernel`**, die `io` nicht bindet. Rust hat kein
//! cross-crate `internal` (C#) — Kapselung über Crate-Grenzen wird über den
//! **Crate-Schnitt** erzwungen, nicht über `pub(crate)`: was `io` nicht sehen darf,
//! liegt außerhalb der Crate, die `io` bindet. `application_kernel` hängt an diesem
//! Kernel, nicht umgekehrt.
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
pub use outbox_message::{OutboxMessage, outbox_payload};
pub use outbox_publisher::OutboxPublisher;
pub use unit_of_work::UnitOfWork;
