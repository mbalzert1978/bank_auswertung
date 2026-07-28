//! Verdrahtung des `einlesen`-Slice in die generische Consumer-Maschinerie:
//! `EinlesenTransactionalHandler` ist der [`TransactionalHandler`] des Slice — es öffnet je
//! Nachricht **eine** Arbeitseinheit, baut den `ReadingHandler` **aus ihr** (Store-/Publish-Ports =
//! die Unit selbst) und committet atomar. [`consumer`] steckt es zu einem [`OutboxConsumer`]
//! zusammen; registriert wird der beim [`Dispatcher`](crate::Dispatcher) unter
//! [`CONSUMED_MESSAGE_TYPE`] (message_type-Auflösung).
//!
//! Die message-unabhängigen Seams (Quelle/Kalender/Geld) und die Unit-Quelle ([`UnitFactory`])
//! reicht der Aufrufer herein. **Alle sind seit
//! [Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md) real** — der Slice
//! läuft end-to-end **ohne einen einzigen Fake**: **Kalender und Geld** seit
//! [Issue 0050](../../../../docs/issues/0050-money-calendar-decode-seams-real.md) echte io-Adapter
//! ([`GermanCalendar`] über `chrono`, [`GermanMoney`] über `rust_decimal` — währungs-bewusst zur
//! Minor Unit skaliert, deren Quelle der [`IsoCurrencies`]-Seam auflöst (`rusty_money`, ISO 4217),
//! [Issue 0055](../../../../docs/issues/0055-waehrungs-bewusster-betrags-decode-minor-unit-per-iso-4217-von-der-domaene-hoch.md)),
//! die **Quelle** seit
//! [Issue 0051](../../../../docs/issues/0051-einlesen-csv-camt-v8-quelle-real-hinter-source-seam.md)
//! der echte CSV-CAMT-v8-Reader ([`CsvCamtV8Source`] über `fs` + `csv`), und die **Unit-Quelle** seit
//! 0053 der reale [`EinlesenSqliteStore`] über `rusqlite`: er backt je Nachricht **eine**
//! `rusqlite`-Transaktion, in der Zeilen-Ablage, `Ingested`-Publish und Ack atomar committen.

mod calendar;
mod currency;
mod money;
mod source;
mod store;

pub use calendar::GermanCalendar;
pub use currency::IsoCurrencies;
pub use money::GermanMoney;
pub use source::CsvCamtV8Source;
pub use store::{EinlesenLines, EinlesenSqliteStore, EinlesenUnit};

use application_kernel::Handler;
use einlesen_adapter::{
    CalendarSeam, CurrencySeam, MoneySeam, ReadCommand, ReadCommandDecoder, ReadError,
    ReadingHandler, SourceSeam, StoreSeam,
};

use crate::{
    MessageId, OutboxConsumer, TransactionError, TransactionalHandler, UnitFactory, run_in_unit,
};

/// Der `message_type`, unter dem der Dispatcher den `einlesen`-Consumer auflöst (der konsumierte
/// Auslöser). Re-exportiert aus dem Adapter — die eine Quelle des Keys liegt am Slice-Eingang.
pub const CONSUMED_MESSAGE_TYPE: &str = einlesen_adapter::CONSUMED_MESSAGE_TYPE;

/// Der [`TransactionalHandler`] von `einlesen`: hält die message-unabhängigen Seams und die
/// [`UnitFactory`]; je `handle` öffnet er eine Unit, baut den `ReadingHandler` **aus ihr** (Store =
/// die Unit, Publish über die Unit) und committet atomar, wenn der Handler gelingt. Weil beide Ports
/// dieselbe committende Unit sind, kann eine gestagete Schreibung die Transaktion nicht überleben.
///
/// Dass die Unit **zusätzlich** der gestagete Speicher dieses Slice ist, verlangt das `impl` unten
/// als `where F::Unit: StoreSeam` — dort, wo der Seam benutzt wird, statt in der uniformen
/// [`UnitFactory`] (ADR-0015, Issue 0077).
pub struct EinlesenTransactionalHandler<'a, F: UnitFactory> {
    source: &'a dyn SourceSeam,
    calendar: &'a dyn CalendarSeam,
    money: &'a dyn MoneySeam,
    currency: &'a dyn CurrencySeam,
    units: F,
}

impl<F: UnitFactory> TransactionalHandler for EinlesenTransactionalHandler<'_, F>
where
    F::Unit: StoreSeam,
{
    type Command = ReadCommand;
    type Decoder = ReadCommandDecoder;
    type Error = ReadError;

    fn handle(
        &self,
        ack: MessageId<'_>,
        command: ReadCommand,
    ) -> Result<(), TransactionError<ReadError>> {
        // `run_in_unit` besitzt den Commit-Rand (Begin/Commit-Ordnung + Fehler-Mapping); dieser Slice
        // reicht nur seinen Handler-Bau **aus der Unit** herein. Store- und Publish-Ports **sind** die
        // Unit — was der Handler staged, committet mit ihr (und dem Ack von `ack`) oder gar nicht.
        run_in_unit(self.units.begin(ack), |unit, publishing| {
            ReadingHandler::new(
                self.source,
                self.calendar,
                self.money,
                self.currency,
                unit,
                publishing,
            )
            .handle(command)
        })
    }
}

/// Steckt `einlesen`s [`TransactionalHandler`] zu einem [`OutboxConsumer`] zusammen.
pub fn consumer<'a, F: UnitFactory>(
    source: &'a dyn SourceSeam,
    calendar: &'a dyn CalendarSeam,
    money: &'a dyn MoneySeam,
    currency: &'a dyn CurrencySeam,
    units: F,
) -> OutboxConsumer<EinlesenTransactionalHandler<'a, F>>
where
    F::Unit: StoreSeam,
{
    OutboxConsumer::new(EinlesenTransactionalHandler {
        source,
        calendar,
        money,
        currency,
        units,
    })
}
