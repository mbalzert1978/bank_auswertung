//! Verdrahtung des `einlesen`-Slice in die generische Consumer-Maschinerie: [`EinlesenTransaction`]
//! ist der [`TransactionalHandler`](crate::TransactionalHandler) des Slice — es öffnet je Nachricht
//! **eine** Arbeitseinheit, baut den [`ReadingHandler`](einlesen_adapter::ReadingHandler) **aus ihr**
//! (Store-/Publish-Ports = die Unit selbst) und committet atomar. [`consumer`] steckt es mit dem
//! [`ReadCommandDecoder`](einlesen_adapter::ReadCommandDecoder) zu einem
//! [`OutboxConsumer`](crate::OutboxConsumer) zusammen; registriert wird der beim
//! [`Dispatcher`](crate::Dispatcher) unter [`CONSUMED_MESSAGE_TYPE`] (message_type-Auflösung).
//!
//! Die message-unabhängigen Seams (Quelle/Kalender/Geld) und die Unit-Quelle ([`Transactions`])
//! reicht der Aufrufer herein — heute der Integrationstest mit Fakes, später die realen io-Adapter
//! (Build-Order Schritt 5: eine `sqlx`-Transaktion pro Nachricht).

use application_kernel::Handler;
use einlesen_adapter::{
    CalendarSeam, MoneySeam, ReadCommand, ReadCommandDecoder, ReadError, ReadingHandler,
    SourceSeam, StoreSeam,
};
use shared_kernel::{OutboxPublisher, UnitOfWork};

use crate::{OutboxConsumer, PublishingPort, TransactionError, TransactionalHandler};

/// Der `message_type`, unter dem der Dispatcher den `einlesen`-Consumer auflöst (der konsumierte
/// Auslöser). Re-exportiert aus dem Adapter — die eine Quelle des Keys liegt am Slice-Eingang.
pub const CONSUMED_MESSAGE_TYPE: &str = einlesen_adapter::CONSUMED_MESSAGE_TYPE;

/// Die **Unit-Quelle** von `einlesen`: `begin` öffnet je Nachricht **eine** Arbeitseinheit, die
/// **zugleich** der gestagete Speicher (`StoreSeam`) **und** die Outbox (`OutboxPublisher`) ist und
/// beides atomar committet (`UnitOfWork`). Weil Store und Publish **dieselbe** Unit sind, die
/// committet, kann eine gestagete Schreibung die committete Transaktion nicht verlassen — das ist die
/// typ-erzwungene Bindung, die das Outbox-Pattern verlangt. Der echte io backt eine Unit mit **einer**
/// `sqlx`-Transaktion; der Test mit einer In-Memory-Unit.
pub trait Transactions {
    /// Die Arbeitseinheit: gestageter Speicher + Outbox in einem, atomar committbar.
    type Unit: UnitOfWork + StoreSeam + OutboxPublisher;

    /// Öffnet die Arbeitseinheit für die anstehende Nachricht.
    fn begin(&self) -> Self::Unit;
}

/// Der [`TransactionalHandler`] von `einlesen`: hält die message-unabhängigen Seams und die
/// [`Transactions`]-Unit-Quelle; je `handle` öffnet er eine Unit, baut den Slice-Handler **aus ihr**
/// und committet atomar, wenn der Handler gelingt.
pub struct EinlesenTransaction<'a, X: Transactions> {
    source: &'a dyn SourceSeam,
    calendar: &'a dyn CalendarSeam,
    money: &'a dyn MoneySeam,
    transactions: X,
}

impl<X: Transactions> TransactionalHandler for EinlesenTransaction<'_, X> {
    type Command = ReadCommand;
    type Error = ReadError;

    fn handle(&self, command: ReadCommand) -> Result<(), TransactionError<ReadError>> {
        let unit = self.transactions.begin();
        // Store- und Publish-Ports **sind** die Unit: was der Handler staged, landet in genau der
        // Unit, die unten committet. Der innere Block gibt die Borrows auf die Unit wieder frei,
        // bevor `commit` sie verbraucht.
        let outcome = {
            let publishing = PublishingPort::new(&unit);
            let handler =
                ReadingHandler::new(self.source, self.calendar, self.money, &unit, &publishing);
            handler.handle(command)
        };
        outcome.map_err(TransactionError::Handler)?;
        unit.commit()
            .map_err(|error| TransactionError::Commit(error.to_string()))
    }
}

/// Steckt `einlesen`s [`TransactionalHandler`] + -Decoder zu einem [`OutboxConsumer`] zusammen.
pub fn consumer<'a, X: Transactions>(
    source: &'a dyn SourceSeam,
    calendar: &'a dyn CalendarSeam,
    money: &'a dyn MoneySeam,
    transactions: X,
) -> OutboxConsumer<EinlesenTransaction<'a, X>, ReadCommandDecoder> {
    OutboxConsumer::new(EinlesenTransaction {
        source,
        calendar,
        money,
        transactions,
    })
}
