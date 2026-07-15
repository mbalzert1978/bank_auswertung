//! Outbox-Mechanik — die **`io`-private** Outbox-Seams der event-getriebenen Pipeline
//! ([ADR-0007](../../../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md),
//! [ADR-0011](../../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)).
//!
//! Hier liegen **nur** die Outbox-Seams, die die Infrastruktur (`io`)
//! sieht/erfüllt — technische Transport-/Ergebnistypen, **keine** Geschäftsdomäne:
//! `OutboxMessage`/`OutboxCommand`/`OutboxError`/`OutboxHandler`/`OutboxPublisher`/
//! `UnitOfWork`. Ergebnis und Fehler einer Nachrichtenverarbeitung trägt Rusts
//! `Result<(), OutboxError>`. Die Seams laufen bewusst **nicht** auf der Domäne
//! zusammen und hängen an **keiner** Dependency (nur `std`).
//!
//! **Kernel-Schnitt nach dem Flip (ADR-0011, Entscheidung D):** Diese Mechanik ist
//! **komplett `io`-privat** — sie lebt als io-internes Modul, nie in einer Crate, die ein
//! Slice-Adapter binden könnte. Der Slice↔`io`-Vertrag
//! (`Command`/`Handler`/`Publisher`/`PublishCommand`/`CommandDecoder`) lebt daneben in der
//! **separaten Crate `application_kernel`**, die ein Slice-Adapter **und** `io` binden. Rust
//! hat kein cross-crate `internal` (C#) — Kapselung über Crate-Grenzen wird über den
//! **Crate-Schnitt** erzwungen, nicht über `pub(crate)`: weil ein Adapter `io` nicht bindet,
//! kann er die Outbox-Mechanik gar nicht importieren.
//!
//! Jeder Vertrag lebt in seiner **eigenen Datei**; dieses Modul zieht die Untermodule nur
//! zusammen und re-exportiert sie flach (`outbox::OutboxHandler` usw.).
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
