//! Die feature-übergreifenden Bausteine der Outbox-Mechanik ([ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):
//! die io-privaten Outbox-Seams selbst ([`outbox`]), der generische [`OutboxConsumer`] über
//! jeden [`TransactionalHandler`], der letzte Outbox-Hop [`PublishingPort`] und der
//! [`Dispatcher`], der eine Nachricht per `message_type` auf ihren Consumer auflöst. Die
//! feature-spezifische Verdrahtung liegt daneben in [`einlesen`](crate::einlesen),
//! [`obfuskierung`](crate::obfuskierung) und [`fenstern`](crate::fenstern).

pub mod outbox;

pub(crate) mod calendar;
mod consumer;
mod dispatcher;
mod publishing;

pub(crate) use consumer::run_in_unit;
pub use consumer::{MessageId, OutboxConsumer, TransactionError, TransactionalHandler};
pub use dispatcher::Dispatcher;
pub use publishing::PublishingPort;
