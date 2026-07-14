//! `io` — die **Infrastruktur** des modularen Monolithen: hier lebt die Outbox-Mechanik, die
//! die Slices seit [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)
//! **nicht** mehr kennen. `io` bindet **beide** Kernel — den io-privaten `shared_kernel` (die
//! Outbox-Seams: `OutboxMessage`/`OutboxError`/`OutboxHandler`/`OutboxPublisher`/`UnitOfWork`)
//! und den Slice↔`io`-Vertrag `application_kernel` (`Handler`/`CommandDecoder`/`Publisher`/…) —
//! sowie die Slice-Adapter, die es verdrahtet.
//!
//! **Die generische Consumer-Maschinerie ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):**
//!
//! - [`OutboxConsumer`] ist der **eine** `shared_kernel::OutboxHandler` über **jeden**
//!   `application_kernel::Handler`: er extrahiert die Envelope, ruft den slice-gelieferten
//!   `CommandDecoder` (Shape Y — nur nackte Strings), dispatcht an den Handler und committet die
//!   **eine** [`UnitOfWork`](shared_kernel::UnitOfWork). Jeder Handler-Fehler ist **terminal** →
//!   Dead-Letter über das **`Display`** des Fehlers (nie `Debug`); `OutboxError` wird nur hier
//!   gebaut, Retry entsteht ausschließlich io-intern.
//! - [`PublishingPort`] macht den **letzten Outbox-Hop**: es bindet
//!   `application_kernel::Publisher` an `shared_kernel::OutboxPublisher`
//!   (`PublishCommand → OutboxCommand`).
//! - [`Dispatcher`] löst eine Nachricht per `message_type` auf ihren Consumer auf.
//! - [`einlesen`] verdrahtet den `einlesen`-Handler + -Decoder in einen [`OutboxConsumer`].
//!
//! **Bewusst draußen (Build-Order Schritt 5):** der echte SQLite-/`sqlx`-Teil (Broker/Stores,
//! CSV-/Datei-Beschaffung). Die Seams (`SourceSeam`/`StoreSeam`/…, `OutboxPublisher`,
//! [`UnitOfWorkFactory`]) erfüllt heute der Integrationstest mit Fakes; die Maschinerie hier ist
//! davon unabhängig und bleibt bei der echten io-Anbindung unverändert.

mod consumer;
mod dispatcher;
mod publishing;

pub mod einlesen;

pub use consumer::{OutboxConsumer, UnitOfWorkFactory};
pub use dispatcher::Dispatcher;
pub use publishing::PublishingPort;
