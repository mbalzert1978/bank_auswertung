//! Verdrahtung des `fenstern`-Slice in die generische Consumer-Maschinerie — **derselbe Bauplan wie
//! [`einlesen`](crate::einlesen) und [`obfuskierung`](crate::obfuskierung)**, nur **zweimal**: der
//! Slice konsumiert **zwei** Kommandotypen, je einen pro einzweckigem Handler (ADR-0013,
//! Entscheidung B/D). Beide bekommen ihren eigenen [`TransactionalHandler`], beide werden beim
//! [`Dispatcher`](crate::Dispatcher) registriert — der Anstoß unter [`START_MESSAGE_TYPE`], der
//! Fenster-Schritt unter [`CONSUMED_MESSAGE_TYPE`] — und beide werden über **dieselbe** uniforme
//! message_type-Auflösung gefahren. Der Slice-Adapter kennt davon nichts (ADR-0011).
//!
//! **Was in die Transaktion gehört und was nicht:** die Unit erfüllt genau die **committenden**
//! Ports (`AnalysisWindowStoreSeam` + `OutboxPublisher` + `UnitOfWork`) — die Fenster-Ablage und
//! **alle** ausgehenden Nachrichten committen gemeinsam oder gar nicht. Genau das trägt die
//! Selbst-Fortsetzung: der `WindowClosed`-Zeiger und das nächste `AdvanceWindow` liegen in
//! **einer** Arbeitseinheit mit dem abgelegten Fenster, sodass nie eine halb-fortgesetzte Kette
//! entsteht. Das **Lesen** (`ObfuscatedWindowSeam`) ist ein **message-unabhängiger** Seam, den der
//! Aufrufer separat hereinreicht — das direkte Gegenstück zu einlesens `source` bzw.
//! obfuskierungs Lese-Quelle; der idempotente Lesevorgang bleibt **außerhalb** der Transaktion und
//! weitet sie nicht.
//!
//! **Die Seitengröße reicht der Aufrufer als [`NonZeroU32`] herein** (io liest sie aus der Config —
//! die Domäne liest keine Datei); der reale toml-Loader
//! [`FensterConfig`](crate::config::FensterConfig) liegt zentral in [`crate::config`]. Sie geht
//! **nur** den Fenster-Schritt an: der Anstoß ist einzweckig und weiß nichts von Seitengrößen.
//!
//! **Alle Seams sind seit [Issue 0057](../../../../docs/issues/0057-fenstern-store-real-und-end-to-end.md)
//! real** — der Slice läuft end-to-end **ohne einen einzigen Fake** (die Replikation des
//! `obfuskierung`-Musters, [Issue 0056](../../../../docs/issues/0056-obfuskierung-store-real-und-end-to-end.md)):
//! die **Unit-Quelle** ist der reale [`FensternSqliteStore`] über `rusqlite` (eine
//! `rusqlite`-Transaktion je Nachricht, in der Ablage, ausgehende Nachrichten und Ack atomar
//! committen), die **Lese-Quelle** und der **Anker-Port** der reale [`ObfuskiertWindowSource`]
//! (liest Seite und Anker-Fragen über die `obfuskiert`-Tabelle).

use std::num::NonZeroU32;

use application_kernel::Handler;
use fenstern_adapter::{
    AdvanceWindowCommand, AdvanceWindowCommandDecoder, AnalysisWindowStoreSeam,
    ObfuscatedWindowSeam, StartAnalysisCommand, StartAnalysisCommandDecoder, StartHandler,
    WindowError, WindowingHandler,
};

use crate::{
    MessageId, OutboxConsumer, TransactionError, TransactionalHandler, UnitFactory, run_in_unit,
};

mod source;
mod store;

pub use source::ObfuskiertWindowSource;
pub use store::{FensternSqliteStore, FensternUnit, FensternWindows};

/// Der `message_type`, unter dem der Dispatcher den **Fenster-Schritt** auflöst (`AdvanceWindow`).
/// Re-exportiert aus dem Adapter — die eine Quelle des Keys liegt am Slice-Eingang.
///
/// Besonderheit dieses Slice: es ist zugleich der Typ, den der Schritt **selbst publiziert** — der
/// Pager reiht sein eigenes nächstes Kommando ein. Genau das *ist* die Selbst-Fortsetzung.
pub const CONSUMED_MESSAGE_TYPE: &str = fenstern_adapter::CONSUMED_MESSAGE_TYPE;

/// Der `message_type`, unter dem der Dispatcher den **Anstoß** auflöst (`StartAnalysis`) — der
/// zweite vom Slice konsumierte Auslöser. Re-exportiert aus dem Adapter.
pub const START_MESSAGE_TYPE: &str = fenstern_adapter::START_MESSAGE_TYPE;

/// Der [`TransactionalHandler`] des **Fenster-Schritts**: hält den message-unabhängigen Lese-Seam,
/// die Seitengröße und die [`UnitFactory`]; je `handle` öffnet er eine Unit, baut den
/// `WindowingHandler` **aus ihr** (Ablage = die Unit, Publish über die Unit) und committet atomar,
/// wenn der Handler gelingt. Der nackte [`WindowError`] ist **terminal** →
/// [`TransactionError::Handler`].
///
/// Dass die Unit **zusätzlich** die gestagete Fenster-Ablage ist, verlangt das `impl` unten als
/// `where F::Unit: AnalysisWindowStoreSeam` — dort, wo der Seam benutzt wird, statt in der uniformen
/// [`UnitFactory`] (ADR-0015, Issue 0077). Der Anstoß daneben legt nichts ab und verlangt darum auch
/// nichts.
pub struct WindowingTransactionalHandler<'a, F: UnitFactory> {
    window_days: NonZeroU32,
    source: &'a dyn ObfuscatedWindowSeam,
    units: F,
}

impl<F: UnitFactory> TransactionalHandler for WindowingTransactionalHandler<'_, F>
where
    F::Unit: AnalysisWindowStoreSeam,
{
    type Command = AdvanceWindowCommand;
    type Decoder = AdvanceWindowCommandDecoder;
    type Error = WindowError;

    fn handle(
        &self,
        ack: MessageId<'_>,
        command: AdvanceWindowCommand,
    ) -> Result<(), TransactionError<WindowError>> {
        // `run_in_unit` besitzt den Commit-Rand (Begin/Commit-Ordnung + Fehler-Mapping); dieser Slice
        // reicht nur seinen Handler-Bau **aus der Unit** herein. Ablage- und Publish-Ports **sind**
        // die Unit; die Lese-Quelle liegt außerhalb (idempotentes Lesen).
        run_in_unit(self.units.begin(ack), |unit, publishing| {
            WindowingHandler::new(self.window_days, self.source, unit, publishing).handle(command)
        })
    }
}

/// Der [`TransactionalHandler`] des **Anstoßes**: derselbe Commit-Rand, nur mit dem einzweckigen
/// `StartHandler` darin. Er kennt **weder Ablage noch Seitengröße** — er liest den Initial-Anker
/// und publiziert das erste Fenster-Kommando. Ein **leerer Bestand** ist kein Fehler: dann committet
/// die Unit schlicht nichts.
pub struct StartTransactionalHandler<'a, F: UnitFactory> {
    source: &'a dyn ObfuscatedWindowSeam,
    units: F,
}

impl<F: UnitFactory> TransactionalHandler for StartTransactionalHandler<'_, F> {
    type Command = StartAnalysisCommand;
    type Decoder = StartAnalysisCommandDecoder;
    type Error = WindowError;

    fn handle(
        &self,
        ack: MessageId<'_>,
        command: StartAnalysisCommand,
    ) -> Result<(), TransactionError<WindowError>> {
        run_in_unit(self.units.begin(ack), |_unit, publishing| {
            StartHandler::new(self.source, publishing).handle(command)
        })
    }
}

/// Steckt den **Fenster-Schritt** zu einem [`OutboxConsumer`] zusammen (registrieren unter
/// [`CONSUMED_MESSAGE_TYPE`]).
pub fn consumer<'a, F: UnitFactory>(
    window_days: NonZeroU32,
    source: &'a dyn ObfuscatedWindowSeam,
    units: F,
) -> OutboxConsumer<WindowingTransactionalHandler<'a, F>>
where
    F::Unit: AnalysisWindowStoreSeam,
{
    OutboxConsumer::new(WindowingTransactionalHandler {
        window_days,
        source,
        units,
    })
}

/// Steckt den **Anstoß** zu einem [`OutboxConsumer`] zusammen (registrieren unter
/// [`START_MESSAGE_TYPE`]).
pub fn start_consumer<'a, F: UnitFactory>(
    source: &'a dyn ObfuscatedWindowSeam,
    units: F,
) -> OutboxConsumer<StartTransactionalHandler<'a, F>> {
    OutboxConsumer::new(StartTransactionalHandler { source, units })
}
