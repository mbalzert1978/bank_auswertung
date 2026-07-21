//! Verdrahtung des `einlesen`-Slice in die generische Consumer-Maschinerie:
//! `EinlesenTransactionalHandler` ist der [`TransactionalHandler`] des Slice — es öffnet je
//! Nachricht **eine** Arbeitseinheit, baut den `ReadingHandler` **aus ihr** (Store-/Publish-Ports =
//! die Unit selbst) und committet atomar. [`consumer`] steckt es zu einem [`OutboxConsumer`]
//! zusammen; registriert wird der beim [`Dispatcher`](crate::Dispatcher) unter
//! [`CONSUMED_MESSAGE_TYPE`] (message_type-Auflösung).
//!
//! Die message-unabhängigen Seams (Quelle/Kalender/Geld) und die Unit-Quelle ([`UnitFactory`])
//! reicht der Aufrufer herein. **Kalender und Geld sind seit
//! [Issue 0050](../../../../docs/issues/0050-money-calendar-decode-seams-real.md) echte, reine
//! io-Adapter** ([`GermanCalendar`] über `chrono`, [`GermanMoney`] über `rust_decimal`); Quelle und
//! Unit-Quelle bleiben bis Build-Order Schritt 5 Fakes (die reale `rusqlite`-Transaktion pro
//! Nachricht + CSV-/Datei-Beschaffung folgen dort).

mod calendar;
mod money;

pub use calendar::GermanCalendar;
pub use money::GermanMoney;

use crate::outbox::{OutboxPublisher, UnitOfWork};
use application_kernel::Handler;
use einlesen_adapter::{
    CalendarSeam, MoneySeam, ReadCommand, ReadCommandDecoder, ReadError, ReadingHandler,
    SourceSeam, StoreSeam,
};

use crate::{OutboxConsumer, TransactionError, TransactionalHandler, run_in_unit};

/// Der `message_type`, unter dem der Dispatcher den `einlesen`-Consumer auflöst (der konsumierte
/// Auslöser). Re-exportiert aus dem Adapter — die eine Quelle des Keys liegt am Slice-Eingang.
pub const CONSUMED_MESSAGE_TYPE: &str = einlesen_adapter::CONSUMED_MESSAGE_TYPE;

/// Die **Unit-Quelle** von `einlesen`: `begin` öffnet je Nachricht **eine** Arbeitseinheit, die
/// **zugleich** der gestagete Speicher (`StoreSeam`) **und** die Outbox (`OutboxPublisher`) ist und
/// beides atomar committet (`UnitOfWork`). Weil dieselbe Unit staged **und** committet, committet
/// eine gestagete Schreibung mit der Nachricht oder gar nicht. Der echte io backt eine Unit mit
/// **einer** `rusqlite`-Transaktion; der Test mit einer In-Memory-Unit.
pub trait UnitFactory {
    /// Die Arbeitseinheit: gestageter Speicher + Outbox in einem, atomar committbar.
    type Unit: UnitOfWork + StoreSeam + OutboxPublisher;

    /// Öffnet die Arbeitseinheit für die anstehende Nachricht.
    fn begin(&self) -> Self::Unit;
}

/// Der [`TransactionalHandler`] von `einlesen`: hält die message-unabhängigen Seams und die
/// [`UnitFactory`]; je `handle` öffnet er eine Unit, baut den `ReadingHandler` **aus ihr** (Store =
/// die Unit, Publish über die Unit) und committet atomar, wenn der Handler gelingt. Weil beide Ports
/// dieselbe committende Unit sind, kann eine gestagete Schreibung die Transaktion nicht überleben.
pub struct EinlesenTransactionalHandler<'a, F: UnitFactory> {
    source: &'a dyn SourceSeam,
    calendar: &'a dyn CalendarSeam,
    money: &'a dyn MoneySeam,
    units: F,
}

impl<F: UnitFactory> TransactionalHandler for EinlesenTransactionalHandler<'_, F> {
    type Command = ReadCommand;
    type Decoder = ReadCommandDecoder;
    type Error = ReadError;

    fn handle(&self, command: ReadCommand) -> Result<(), TransactionError<ReadError>> {
        // `run_in_unit` besitzt den Commit-Rand (Begin/Commit-Ordnung + Fehler-Mapping); dieser Slice
        // reicht nur seinen Handler-Bau **aus der Unit** herein. Store- und Publish-Ports **sind** die
        // Unit — was der Handler staged, committet mit ihr oder gar nicht.
        run_in_unit(self.units.begin(), |unit, publishing| {
            ReadingHandler::new(self.source, self.calendar, self.money, unit, publishing)
                .handle(command)
        })
    }
}

/// Steckt `einlesen`s [`TransactionalHandler`] zu einem [`OutboxConsumer`] zusammen.
pub fn consumer<'a, F: UnitFactory>(
    source: &'a dyn SourceSeam,
    calendar: &'a dyn CalendarSeam,
    money: &'a dyn MoneySeam,
    units: F,
) -> OutboxConsumer<EinlesenTransactionalHandler<'a, F>> {
    OutboxConsumer::new(EinlesenTransactionalHandler {
        source,
        calendar,
        money,
        units,
    })
}
