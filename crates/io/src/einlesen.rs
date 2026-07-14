//! Verdrahtung des `einlesen`-Slice in die generische Consumer-Maschinerie: sein
//! [`ReadingHandler`](einlesen_adapter::ReadingHandler) + [`ReadCommandDecoder`](einlesen_adapter::ReadCommandDecoder)
//! werden zu einem [`OutboxConsumer`](crate::OutboxConsumer) über den injizierten Seams
//! zusammengesteckt. Der Consumer wird beim [`Dispatcher`](crate::Dispatcher) unter
//! [`CONSUMED_MESSAGE_TYPE`] registriert (message_type-Auflösung). Die Seams (Quelle/Kalender/
//! Geld/Speicher/`OutboxPublisher`/Unit-of-Work) reicht der Aufrufer herein — heute der
//! Integrationstest mit Fakes, später die realen io-Adapter (Build-Order Schritt 5).

use einlesen_adapter::{
    CalendarSeam, MoneySeam, ReadCommandDecoder, ReadingHandler, SourceSeam, StoreSeam,
};
use shared_kernel::OutboxPublisher;

use crate::{OutboxConsumer, PublishingPort, UnitOfWorkFactory};

/// Der `message_type`, unter dem der Dispatcher den `einlesen`-Consumer auflöst (der konsumierte
/// Auslöser). Re-exportiert aus dem Adapter — die eine Quelle des Keys liegt am Slice-Eingang.
pub const CONSUMED_MESSAGE_TYPE: &str = einlesen_adapter::CONSUMED_MESSAGE_TYPE;

/// Steckt den `einlesen`-Handler + -Decoder zu einem [`OutboxConsumer`] über den injizierten
/// Seams zusammen. Der [`PublishingPort`] bindet den Ausgang an den `OutboxPublisher`; die
/// [`UnitOfWorkFactory`] öffnet die atomare Arbeitseinheit je Nachricht.
pub fn consumer<'a, O, U>(
    source: &'a dyn SourceSeam,
    calendar: &'a dyn CalendarSeam,
    money: &'a dyn MoneySeam,
    store: &'a dyn StoreSeam,
    publisher: &'a PublishingPort<'a, O>,
    unit_of_work: U,
) -> OutboxConsumer<ReadingHandler<'a>, U, ReadCommandDecoder>
where
    O: OutboxPublisher,
    U: UnitOfWorkFactory,
{
    OutboxConsumer::new(
        ReadingHandler::new(source, calendar, money, store, publisher),
        unit_of_work,
    )
}
