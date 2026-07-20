//! Verdrahtung des `auswerten`-Slice in die generische Consumer-Maschinerie — **derselbe Bauplan
//! wie [`fenstern`](crate::fenstern)** und aus demselben Grund **zweimal**: der Slice trägt **zwei
//! Reichweiten**, je einen einzweckigen Handler (ADR-0013, Entscheidung E). Beide bekommen ihren
//! eigenen [`TransactionalHandler`], beide werden beim [`Dispatcher`](crate::Dispatcher)
//! registriert — die pro-Fenster-Reichweite unter [`CONSUMED_MESSAGE_TYPE`], die
//! fenster-übergreifende unter [`CONSUMED_HISTORY_MESSAGE_TYPE`] — und beide werden über
//! **dieselbe** uniforme message_type-Auflösung gefahren. Der Slice-Adapter kennt davon nichts
//! (ADR-0011): er bindet keinen Outbox-Typ, die Consumer-Rolle lebt allein hier.
//!
//! **Was in die Transaktion gehört und was nicht:** die Unit erfüllt genau die **committenden**
//! Ports ([`EvaluationStoreSeam`] + [`OutboxPublisher`] + [`UnitOfWork`]). Die **Lese**-Seams
//! ([`AnalysisWindowSeam`]/[`OccurrenceSeam`]) und das Orakel sind **message-unabhängig** und
//! werden vom Aufrufer separat hereingereicht — das direkte Gegenstück zu `fenstern`s Lese-Quelle;
//! idempotentes Lesen bleibt **außerhalb** der Transaktion und weitet sie nicht.
//!
//! **Dieser Slice publiziert nichts.** Er ist das **Ende** der Pipeline (bis der `report` gebaut
//! wird): keine der beiden Reichweiten stößt etwas stromabwärts an. Der [`PublishingPort`] der
//! Unit bleibt darum ungenutzt — die Arbeitseinheit ist deshalb aber keine andere. Sie umspannt
//! weiterhin Ack und Ablage, und weil sie derselbe Commit-Rand ist wie überall, wächst ein späterer
//! Publish (der `report`-Anstoß) ohne Umbau hinein.
//!
//! **Die Eigene-Konten-Config reicht der Aufrufer als Wert herein** (io liest sie aus der Config —
//! der Slice liest keine Datei, ADR-0013 Entscheidung F), heute der Integrationstest mit Fakes,
//! später der reale Loader ([Issue 0002](../../../../docs/issues/0002-eigene-konten-config-schema-und-loader.md)).
//! Sie geht **nur** die pro-Fenster-Reichweite an: die fenster-übergreifende ordnet keine
//! Gegenkonten ein, sie sammelt Vorkommen ein.

use application_kernel::Handler;
use auswerten_adapter::{
    AnalysisWindowSeam, CategorizerSeam, CategoryRegistrySeam, EvaluateHistoryCommand,
    EvaluateHistoryCommandDecoder, EvaluateWindowCommand, EvaluateWindowCommandDecoder,
    EvaluationError, EvaluationStoreSeam, HistoryEvaluationHandler, OccurrenceRow, OccurrenceSeam,
    OwnAccountsConfig, WindowEvaluationHandler,
};

use crate::outbox::{OutboxPublisher, UnitOfWork};
use crate::{OutboxConsumer, TransactionError, TransactionalHandler, run_in_unit};

/// Der `message_type`, unter dem der Dispatcher die **pro-Fenster**-Reichweite auflöst
/// (`FensterGeschlossen`). Re-exportiert aus dem Adapter — die eine Quelle des Keys liegt am
/// Slice-Eingang. Es ist derselbe Key, den [`fenstern`](crate::fenstern) je materialisiertem Fenster
/// publiziert; **dass** die beiden zusammenpassen, ist die ganze Kopplung zwischen den Slices.
pub const CONSUMED_MESSAGE_TYPE: &str = auswerten_adapter::CONSUMED_MESSAGE_TYPE;

/// Der `message_type`, unter dem der Dispatcher die **fenster-übergreifende** Reichweite auflöst
/// (`HistorieVollständig`). Ein **anderer** Key als [`CONSUMED_MESSAGE_TYPE`] — ein geschlossenes
/// Fenster erreicht diesen Handler also nie, und die Query läuft **einmal** am Ende des Laufs statt
/// je Fenster.
pub const CONSUMED_HISTORY_MESSAGE_TYPE: &str = auswerten_adapter::CONSUMED_HISTORY_MESSAGE_TYPE;

/// Die **Unit-Quelle** von `auswerten`: `begin` öffnet je Nachricht **eine** Arbeitseinheit, die
/// **zugleich** die gestagete Ergebnis-Ablage ([`EvaluationStoreSeam`]) **und** die Outbox
/// ([`OutboxPublisher`]) ist und beides atomar committet ([`UnitOfWork`]). Spiegel von `fenstern`s
/// `UnitFactory`; der echte io backt eine Unit mit **einer** `sqlx`-Transaktion.
///
/// **Eine** Unit-Quelle für **beide** Reichweiten: es gibt eine Ablage und eine Outbox, also auch
/// eine Transaktions-Quelle. Dass die fenster-übergreifende Reichweite nichts ablegt, ändert daran
/// nichts — ihre Arbeitseinheit umspannt dann eben nur den Ack.
pub trait UnitFactory {
    /// Die Arbeitseinheit: gestagete Ergebnis-Ablage + Outbox in einem, atomar committbar.
    type Unit: UnitOfWork + EvaluationStoreSeam + OutboxPublisher;

    /// Öffnet die Arbeitseinheit für die anstehende Nachricht.
    fn begin(&self) -> Self::Unit;
}

/// Der **Abnehmer** der fenster-übergreifenden Reichweite — die io-Seite der einen Stelle, an der
/// dieser Slice aus dem `Result<(), _>`-Muster ausbricht.
///
/// **Warum es ihn gibt:** diese Reichweite **gibt ihr Ergebnis zurück, statt es abzulegen** (es gibt
/// kein Vorkommens-/Fold-Log — die Aggregate sind die alleinige Wahrheit, und die Idempotenz folgt
/// daraus statt aus Dedupe-Logik). Der Consumer-Vertrag ist aber `Result<(), _>`: ohne einen
/// Abnehmer verpuffte die Response schlicht. Statt den **generischen** Vertrag aller vier Consumer
/// um einen Rückgabewert wachsen zu lassen — den drei von ihnen nie füllten — bekommt **diese eine**
/// Reichweite ihren direkten Abnehmer, genau so, wie der Slice-Adapter es vorzeichnet. Der reale
/// Abnehmer ist der Wiederkehr-Analyzer
/// ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)).
///
/// **Unfehlbar, wie [`OutboxPublisher::publish`]:** ein fehlbarer Abnehmer bräuchte eine Abbildung
/// seines Fehlers auf den **Domänen**-Fehler des Slice, und den besitzt allein der Slice — io kann
/// ihn nicht konstruieren, ohne sich den Meldungssatz anzueignen.
///
/// **Zustellung *innerhalb* der Arbeitseinheit, vor dem Commit:** scheitert der Commit, wird die
/// Nachricht erneut zugestellt und der Abnehmer sieht dasselbe Ergebnis noch einmal. Das ist
/// at-least-once und hier unschädlich — die Query über unveränderliche Snapshots antwortet auf
/// erneute Zustellung identisch.
pub trait OccurrenceSink {
    /// Nimmt die Vorkommen der ganzen Zeitachse des Laufs entgegen.
    fn receive(&self, occurrences: Vec<OccurrenceRow>);
}

/// Der [`TransactionalHandler`] der **pro-Fenster**-Reichweite: hält die message-unabhängigen Seams
/// (Lese-Quelle, Kategorie-Register, Orakel), die Eigene-Konten-Config und die [`UnitFactory`]; je
/// `handle` öffnet er eine Unit, baut den `WindowEvaluationHandler` **aus ihr** (Ablage = die Unit)
/// und committet atomar, wenn der Handler gelingt. Der nackte [`EvaluationError`] ist **terminal** →
/// [`TransactionError::Handler`].
pub struct WindowEvaluationTransactionalHandler<'a, F: UnitFactory> {
    source: &'a dyn AnalysisWindowSeam,
    registry: &'a dyn CategoryRegistrySeam,
    categorizer: &'a dyn CategorizerSeam,
    own_accounts: OwnAccountsConfig,
    units: F,
}

impl<F: UnitFactory> TransactionalHandler for WindowEvaluationTransactionalHandler<'_, F> {
    type Command = EvaluateWindowCommand;
    type Decoder = EvaluateWindowCommandDecoder;
    type Error = EvaluationError;

    fn handle(
        &self,
        command: EvaluateWindowCommand,
    ) -> Result<(), TransactionError<EvaluationError>> {
        // `run_in_unit` besitzt den Commit-Rand (Begin/Commit-Ordnung + Fehler-Mapping); dieser Slice
        // reicht nur seinen Handler-Bau **aus der Unit** herein. Der Ablage-Port **ist** die Unit;
        // die Lese-Seams und das Orakel liegen außerhalb (idempotentes Lesen bzw. Egress).
        run_in_unit(self.units.begin(), |unit, _publishing| {
            WindowEvaluationHandler::new(
                self.source,
                unit,
                self.registry,
                self.categorizer,
                self.own_accounts.clone(),
            )
            .handle(command)
        })
    }
}

/// Der [`TransactionalHandler`] der **fenster-übergreifenden** Reichweite: derselbe Commit-Rand, nur
/// mit dem einzweckigen `HistoryEvaluationHandler` darin. Er kennt **weder Ablage noch Config** — er
/// führt eine Query über alle Fenster des Laufs und reicht ihr Ergebnis an den [`OccurrenceSink`]
/// weiter. Seine Arbeitseinheit umspannt darum allein den Ack.
pub struct HistoryEvaluationTransactionalHandler<'a, F: UnitFactory, S: OccurrenceSink> {
    source: &'a dyn OccurrenceSeam,
    sink: S,
    units: F,
}

impl<F: UnitFactory, S: OccurrenceSink> TransactionalHandler
    for HistoryEvaluationTransactionalHandler<'_, F, S>
{
    type Command = EvaluateHistoryCommand;
    type Decoder = EvaluateHistoryCommandDecoder;
    type Error = EvaluationError;

    fn handle(
        &self,
        command: EvaluateHistoryCommand,
    ) -> Result<(), TransactionError<EvaluationError>> {
        run_in_unit(self.units.begin(), |_unit, _publishing| {
            HistoryEvaluationHandler::new(self.source)
                .handle(command)
                .map(|occurrences| self.sink.receive(occurrences))
        })
    }
}

/// Steckt die **pro-Fenster**-Reichweite zu einem [`OutboxConsumer`] zusammen (registrieren unter
/// [`CONSUMED_MESSAGE_TYPE`]).
pub fn consumer<'a, F: UnitFactory>(
    source: &'a dyn AnalysisWindowSeam,
    registry: &'a dyn CategoryRegistrySeam,
    categorizer: &'a dyn CategorizerSeam,
    own_accounts: OwnAccountsConfig,
    units: F,
) -> OutboxConsumer<WindowEvaluationTransactionalHandler<'a, F>> {
    OutboxConsumer::new(WindowEvaluationTransactionalHandler {
        source,
        registry,
        categorizer,
        own_accounts,
        units,
    })
}

/// Steckt die **fenster-übergreifende** Reichweite zu einem [`OutboxConsumer`] zusammen
/// (registrieren unter [`CONSUMED_HISTORY_MESSAGE_TYPE`]).
pub fn history_consumer<'a, F: UnitFactory, S: OccurrenceSink>(
    source: &'a dyn OccurrenceSeam,
    sink: S,
    units: F,
) -> OutboxConsumer<HistoryEvaluationTransactionalHandler<'a, F, S>> {
    OutboxConsumer::new(HistoryEvaluationTransactionalHandler {
        source,
        sink,
        units,
    })
}
