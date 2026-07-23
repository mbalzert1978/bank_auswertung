//! Verdrahtung des `obfuskierung`-Slice in die generische Consumer-Maschinerie — **Zeile für Zeile
//! parallel zu [`einlesen`](crate::einlesen)**: `ObfuskierungTransactionalHandler` ist der
//! [`TransactionalHandler`] des Slice — es öffnet je Nachricht **eine** Arbeitseinheit, baut den
//! `ObfuscatingHandler` **aus ihr** (Store-/Publish-Ports = die Unit selbst) und committet atomar.
//! [`consumer`] steckt es zu einem [`OutboxConsumer`] zusammen; registriert wird der beim
//! [`Dispatcher`](crate::Dispatcher) unter [`CONSUMED_MESSAGE_TYPE`] (message_type-Auflösung).
//!
//! **Was in die Transaktion gehört und was nicht:** die Unit erfüllt genau die **committenden** Ports
//! (`ObfuscatedStoreSeam` + `OutboxPublisher` + `UnitOfWork`) — Ablage und Nachricht committen
//! gemeinsam oder gar nicht. Das **Lesen** (`IngestedSourceSeam`) und der reine `ObfuscatorSeam` sind
//! **message-unabhängige Seams**, die der Aufrufer separat hereinreicht — die direkten Gegenparts zu
//! einlesens `source` bzw. `calendar`/`money`; der idempotente Lesevorgang (die einlesen-Zeile ist
//! append-only/unveränderlich) bleibt **außerhalb** der Transaktion und weitet sie nicht. Den Seed
//! reicht der Aufrufer als rohen `u64` herein (io liest ihn aus der Config).
//!
//! **Alle Seams sind seit [Issue 0056](../../../../docs/issues/0056-obfuskierung-store-real-und-end-to-end.md)
//! real** — der Slice läuft end-to-end **ohne einen einzigen Fake** (die Replikation des
//! `einlesen`-Musters, [Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md),
//! über die Datenschutz-Grenze): die **Unit-Quelle** ist der reale [`ObfuskierungSqliteStore`] über
//! `rusqlite` (eine `rusqlite`-Transaktion je Nachricht, in der Ablage, `Obfuscated`-Publish und Ack
//! atomar committen), die **Lese-Quelle** die reale [`EinlesenRowSource`] (liest die
//! `einlesen_lines`-Zeile per ID) und der **Obfuskator** der reale [`SeededObfuscator`] (seed-gekeytes
//! `siphasher`).

mod obfuscator;
mod source;
mod store;

pub use obfuscator::SeededObfuscator;
pub use source::EinlesenRowSource;
pub use store::{ObfuskierungSqliteStore, ObfuskierungUnit};

use crate::outbox::{OutboxPublisher, UnitOfWork};
use application_kernel::Handler;
use obfuskierung_adapter::{
    IngestedSourceSeam, ObfuscateCommand, ObfuscateCommandDecoder, ObfuscatedStoreSeam,
    ObfuscatingHandler, ObfuscatorSeam, SourceError,
};

use crate::{MessageId, OutboxConsumer, TransactionError, TransactionalHandler, run_in_unit};

/// Der `message_type`, unter dem der Dispatcher den `obfuskierung`-Consumer auflöst (der konsumierte
/// Auslöser, `Ingested`). Re-exportiert aus dem Adapter — die eine Quelle des Keys liegt am
/// Slice-Eingang.
pub const CONSUMED_MESSAGE_TYPE: &str = obfuskierung_adapter::CONSUMED_MESSAGE_TYPE;

/// Die **Unit-Quelle** von `obfuskierung`: `begin` öffnet je Nachricht **eine** Arbeitseinheit, die
/// **zugleich** die gestagete `obfuskiert`-Ablage ([`ObfuscatedStoreSeam`]) **und** die Outbox
/// ([`OutboxPublisher`]) ist und beides atomar committet ([`UnitOfWork`]). Weil dieselbe Unit staged
/// **und** committet, committet eine gestagete Schreibung mit der Nachricht oder gar nicht. Spiegel
/// von einlesens `UnitFactory`; der echte io backt eine Unit mit **einer** `rusqlite`-Transaktion.
pub trait UnitFactory {
    /// Die Arbeitseinheit: gestagete Ablage + Outbox in einem, atomar committbar.
    type Unit: UnitOfWork + ObfuscatedStoreSeam + OutboxPublisher;

    /// Öffnet die Arbeitseinheit für die Nachricht `ack`, die sie beim Commit als verarbeitet
    /// markiert. Die Ack-ID kommt **explizit** als Argument (nicht aus ambient Zustand).
    fn begin(&self, ack: MessageId<'_>) -> Self::Unit;
}

/// Der [`TransactionalHandler`] von `obfuskierung`: hält die message-unabhängigen Seams (Lese-Quelle
/// und Obfuskator) und den Seed sowie die [`UnitFactory`]; je `handle` öffnet er eine Unit, baut den
/// `ObfuscatingHandler` **aus ihr** (Store = die Unit, Publish über die Unit) und committet atomar,
/// wenn der Handler gelingt. Weil beide committenden Ports dieselbe Unit sind, kann eine gestagete
/// Schreibung die Transaktion nicht überleben. Der nackte [`SourceError`] ist **terminal** →
/// [`TransactionError::Handler`].
pub struct ObfuskierungTransactionalHandler<'a, F: UnitFactory> {
    source: &'a dyn IngestedSourceSeam,
    obfuscator: &'a dyn ObfuscatorSeam,
    seed: u64,
    units: F,
}

impl<F: UnitFactory> TransactionalHandler for ObfuskierungTransactionalHandler<'_, F> {
    type Command = ObfuscateCommand;
    type Decoder = ObfuscateCommandDecoder;
    type Error = SourceError;

    fn handle(
        &self,
        ack: MessageId<'_>,
        command: ObfuscateCommand,
    ) -> Result<(), TransactionError<SourceError>> {
        // `run_in_unit` besitzt den Commit-Rand (Begin/Commit-Ordnung + Fehler-Mapping); dieser Slice
        // reicht nur seinen Handler-Bau **aus der Unit** herein. Store- und Publish-Ports **sind** die
        // Unit; Quelle/Obfuskator liegen außerhalb (idempotentes Lesen).
        run_in_unit(self.units.begin(ack), |unit, publishing| {
            ObfuscatingHandler::new(self.seed, self.source, self.obfuscator, unit, publishing)
                .handle(command)
        })
    }
}

/// Steckt `obfuskierung`s [`TransactionalHandler`] zu einem [`OutboxConsumer`] zusammen.
pub fn consumer<'a, F: UnitFactory>(
    source: &'a dyn IngestedSourceSeam,
    obfuscator: &'a dyn ObfuscatorSeam,
    seed: u64,
    units: F,
) -> OutboxConsumer<ObfuskierungTransactionalHandler<'a, F>> {
    OutboxConsumer::new(ObfuskierungTransactionalHandler {
        source,
        obfuscator,
        seed,
        units,
    })
}
