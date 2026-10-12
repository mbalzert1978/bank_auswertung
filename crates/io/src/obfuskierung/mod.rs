//! Verdrahtung des `obfuskierung`-Slice in die generische Consumer-Maschinerie — **Zeile für Zeile
//! parallel zu [`einlesen`](crate::einlesen)**: `ObfuskierungTransactionalHandler` ist der
//! [`TransactionalHandler`] des Slice — es öffnet je Nachricht **eine** Arbeitseinheit, baut den
//! `ObfuscatingHandler` **aus ihr** (Store-/Publish-Ports = die Unit selbst) und committet atomar.
//! [`consumer`] steckt es zu einem [`OutboxConsumer`] zusammen; registriert wird der beim
//! [`Dispatcher`](crate::Dispatcher) unter [`CONSUMED_MESSAGE_TYPE`] (message_type-Auflösung).
//!
//! **Was in die Transaktion gehört und was nicht:** die Unit erfüllt genau die **committenden** Ports
//! (`ObfuscatedStoreSeam` + `OutboxPublisher` + `UnitOfWork`) — Ablage und Nachricht committen
//! gemeinsam oder gar nicht. Das **Lesen** (`IngestedSourceSeam`, dazu die Namensliste
//! `KnownPersonsSeam`) und der reine `ObfuscatorSeam` sind **message-unabhängige Seams**, die der Aufrufer separat hereinreicht — die direkten Gegenparts zu
//! einlesens `source` bzw. `calendar`/`money`; der idempotente Lesevorgang (die einlesen-Zeile ist
//! append-only/unveränderlich) bleibt **außerhalb** der Transaktion und weitet sie nicht. Den Seed
//! reicht der Aufrufer als rohen `u64` herein; der reale toml-Loader
//! [`SeedConfig`](crate::config::SeedConfig) liegt zentral in [`crate::config`] und liest ihn aus
//! der Config.
//!
//! **Alle Seams sind seit [Issue 0056](../../../../docs/issues/0056-obfuskierung-store-real-und-end-to-end.md)
//! real** — der Slice läuft end-to-end **ohne einen einzigen Fake** (die Replikation des
//! `einlesen`-Musters, [Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md),
//! über die Datenschutz-Grenze): die **Unit-Quelle** ist der reale [`ObfuskierungSqliteStore`] über
//! `rusqlite` (eine `rusqlite`-Transaktion je Nachricht, in der Ablage, `Obfuscated`-Publish und Ack
//! atomar committen), die **Lese-Quelle** die reale [`EinlesenRowSource`] (liest die
//! `einlesen_lines`-Zeile per ID), der **Obfuskator** der reale [`SeededObfuscator`] (seed-gekeytes
//! `siphasher`) und — seit [Issue 0010](../../../../docs/issues/0010-scrubbing-regeln-fuer-obfuskiertertext.md)
//! — die **Namensliste** des Freitext-Engpasses der reale [`KnownPersonNames`] (Empfänger ohne
//! Gläubiger-ID aus `einlesen_lines` plus die lokale `personen.toml`).

mod obfuscator;
mod persons;
mod source;
mod store;

pub use obfuscator::SeededObfuscator;
pub(crate) use obfuscator::{SeedMarker, seed_marker};
pub use persons::KnownPersonNames;
pub use source::EinlesenRowSource;
pub use store::{ObfuskiertBookings, ObfuskierungSqliteStore, ObfuskierungUnit};

use application_kernel::Handler;
use obfuskierung_adapter::{
    IngestedSourceSeam, KnownPersonsSeam, ObfuscateCommand, ObfuscateCommandDecoder,
    ObfuscatedStoreSeam, ObfuscatingHandler, ObfuscationError, ObfuscatorSeam,
};

use crate::{
    MessageId, OutboxConsumer, TransactionError, TransactionalHandler, UnitFactory, run_in_unit,
};

/// Der `message_type`, unter dem der Dispatcher den `obfuskierung`-Consumer auflöst (der konsumierte
/// Auslöser, `Ingested`). Re-exportiert aus dem Adapter — die eine Quelle des Keys liegt am
/// Slice-Eingang.
pub const CONSUMED_MESSAGE_TYPE: &str = obfuskierung_adapter::CONSUMED_MESSAGE_TYPE;

/// Der [`TransactionalHandler`] von `obfuskierung`: hält die message-unabhängigen Seams (Lese-Quelle,
/// Obfuskator und Namensliste) und den Seed sowie die [`UnitFactory`]; je `handle` öffnet er eine Unit, baut den
/// `ObfuscatingHandler` **aus ihr** (Store = die Unit, Publish über die Unit) und committet atomar,
/// wenn der Handler gelingt. Weil beide committenden Ports dieselbe Unit sind, kann eine gestagete
/// Schreibung die Transaktion nicht überleben. Der nackte [`ObfuscationError`] ist **terminal** →
/// [`TransactionError::Handler`].
///
/// Dass die Unit **zusätzlich** die gestagete `obfuskiert`-Ablage ist, verlangt das `impl` unten als
/// `where F::Unit: ObfuscatedStoreSeam` — dort, wo der Seam benutzt wird, statt in der uniformen
/// [`UnitFactory`] (ADR-0015, Issue 0077).
pub struct ObfuskierungTransactionalHandler<'a, F: UnitFactory> {
    source: &'a dyn IngestedSourceSeam,
    obfuscator: &'a dyn ObfuscatorSeam,
    persons: &'a dyn KnownPersonsSeam,
    seed: u64,
    units: F,
}

impl<F: UnitFactory> TransactionalHandler for ObfuskierungTransactionalHandler<'_, F>
where
    F::Unit: ObfuscatedStoreSeam,
{
    type Command = ObfuscateCommand;
    type Decoder = ObfuscateCommandDecoder;
    type Error = ObfuscationError;

    fn handle(
        &self,
        ack: MessageId<'_>,
        command: ObfuscateCommand,
    ) -> Result<(), TransactionError<ObfuscationError>> {
        // `run_in_unit` besitzt den Commit-Rand (Begin/Commit-Ordnung + Fehler-Mapping); dieser Slice
        // reicht nur seinen Handler-Bau **aus der Unit** herein. Store- und Publish-Ports **sind** die
        // Unit; Quelle/Obfuskator/Namensliste liegen außerhalb (idempotentes Lesen).
        run_in_unit(self.units.begin(ack), |unit, publishing| {
            ObfuscatingHandler::new(
                self.seed,
                self.source,
                self.obfuscator,
                self.persons,
                unit,
                publishing,
            )
            .handle(command)
        })
    }
}

/// Steckt `obfuskierung`s [`TransactionalHandler`] zu einem [`OutboxConsumer`] zusammen.
pub fn consumer<'a, F: UnitFactory>(
    source: &'a dyn IngestedSourceSeam,
    obfuscator: &'a dyn ObfuscatorSeam,
    persons: &'a dyn KnownPersonsSeam,
    seed: u64,
    units: F,
) -> OutboxConsumer<ObfuskierungTransactionalHandler<'a, F>>
where
    F::Unit: ObfuscatedStoreSeam,
{
    OutboxConsumer::new(ObfuskierungTransactionalHandler {
        source,
        obfuscator,
        persons,
        seed,
        units,
    })
}
