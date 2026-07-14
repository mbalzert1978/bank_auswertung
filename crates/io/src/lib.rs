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
//!   [`TransactionalHandler`]: er extrahiert die Envelope, ruft den slice-gelieferten
//!   `CommandDecoder` (Shape Y — nur nackte Strings) und dispatcht das Kommando an den
//!   `TransactionalHandler`, der **eine** `UnitOfWork` öffnet, den Slice-Handler **aus ihr** baut
//!   (Store und Publish sind dieselbe committende Unit — eine gestagete Schreibung committet mit
//!   der Nachricht oder gar nicht) und committet. Jeder Handler-Fehler ist **terminal** →
//!   Dead-Letter über das **`Display`** des Fehlers (nie `Debug`). Den `OutboxError` der
//!   Nachrichtenverarbeitung baut der Consumer (Retry entsteht ausschließlich io-intern beim
//!   Commit); die Auflösungs-Ebene baut ihr eigenes `Critical` im [`Dispatcher`].
//! - [`PublishingPort`] macht den **letzten Outbox-Hop**: es bindet
//!   `application_kernel::Publisher` an `shared_kernel::OutboxPublisher`
//!   (`PublishCommand → OutboxCommand`). **Heute pointer-only:** ein Kommando trägt nur
//!   `message_type` + `partition_key` (Tabelle = Wahrheit, Message = Zeiger), kein `content`.
//!   Ein erster nutzlast-tragender Slice erweitert den Outbox-Vertrag um einen Content-/serde-
//!   Seam — dann ist es kein reiner Zeiger mehr.
//! - [`Dispatcher`] löst eine Nachricht per `message_type` auf ihren Consumer auf.
//! - [`einlesen`] verdrahtet den `einlesen`-Handler + -Decoder in einen [`OutboxConsumer`].
//!
//! **Bewusst draußen (Build-Order Schritt 5):** der echte SQLite-/`sqlx`-Teil (Broker/Stores,
//! CSV-/Datei-Beschaffung). Die Seams (`SourceSeam`/`StoreSeam`/…, `OutboxPublisher`,
//! [`einlesen::UnitFactory`]) erfüllt heute der Integrationstest mit Fakes; die Maschinerie hier
//! ist davon unabhängig und bleibt bei der echten io-Anbindung unverändert.

mod consumer;
mod dispatcher;
mod publishing;

pub mod einlesen;

pub use consumer::{OutboxConsumer, TransactionError, TransactionalHandler};
pub use dispatcher::Dispatcher;
pub use publishing::PublishingPort;
