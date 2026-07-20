use super::outbox::{OutboxCommand, OutboxPublisher};
use application_kernel::{PublishCommand, Publisher};
use einlesen_adapter::{IngestedMessage, PublishSeam, PublishUnavailable};
use fenstern_adapter::{
    AdvanceWindowMessage, HistoryCompleteMessage, PublishUnavailable as WindowPublishUnavailable,
    WindowClosedMessage, WindowPublishSeam,
};

/// Der **letzte Outbox-Hop**: reicht ein veröffentlichtes adapter-seitiges Kommando an den
/// io-privaten [`OutboxPublisher`](super::outbox::OutboxPublisher) durch — die eine Stelle, an der `PublishCommand`
/// (adapter-sichtbar) auf `OutboxCommand` (io-privat) trifft. Er erfüllt den Publish-Ausgang jedes
/// Slice: `einlesen`s fehlbaren [`PublishSeam`], `fenstern`s dreiausgängigen [`WindowPublishSeam`]
/// und (bis Issue 0028) obfuskierungs `application_kernel::Publisher`.
///
/// Generisch über den konkreten `OutboxPublisher`, weil dessen `publish<T>` eine generische
/// Methode ist (nicht objekt-sicher).
pub struct PublishingPort<'a, O: OutboxPublisher> {
    outbox: &'a O,
}

impl<'a, O: OutboxPublisher> PublishingPort<'a, O> {
    /// Bindet den Ausgang an den gegebenen `OutboxPublisher` (io/Fake).
    pub fn new(outbox: &'a O) -> Self {
        Self { outbox }
    }
}

impl<O: OutboxPublisher, C: PublishCommand> Publisher<C> for PublishingPort<'_, O> {
    fn publish(&self, command: C) {
        self.outbox.publish(OutboxHop { command });
    }
}

impl<O: OutboxPublisher> PublishSeam for PublishingPort<'_, O> {
    /// Reicht `einlesen`s „Eingelesen"-Kommando in die Outbox durch. Der [`PublishSeam`]-Vertrag
    /// ist **bewusst fehlbar** — ein Transport, der beim Einreihen scheitern kann, mappt seinen
    /// Fehler auf [`PublishUnavailable`] (→ `SinkError::PublishFailed`), ohne dass der Slice sich
    /// ändert. **Der heutige Transport scheitert hier nicht:** das Einreihen *staged* nur in die
    /// offene Arbeitseinheit (`OutboxPublisher::publish`, infallibel) — die eigentliche, fehlbare
    /// Schreibung passiert erst am **Commit-Rand** (`UnitOfWork::commit`, siehe `run_in_unit`), wo
    /// ein Fehlschlag io-intern als Commit-/Retry-Fall erscheint, nicht als terminaler
    /// Publish-Fehler. Darum heute `Ok(())`.
    fn publish(&self, message: IngestedMessage) -> Result<(), PublishUnavailable> {
        self.outbox.publish(OutboxHop { command: message });
        Ok(())
    }
}

/// Erfüllt `fenstern`s **einen** Publish-Seam mit seinen drei Ausgängen: alle drei Nachrichten gehen
/// in **dieselbe** Outbox und damit in **dieselbe** Arbeitseinheit — genau daran hängt, dass nie eine
/// halb-fortgesetzte Kette entsteht (ADR-0013, Entscheidung B).
///
/// **Der heutige Transport scheitert hier nicht** (wie bei `einlesen`s [`PublishSeam`]): das
/// Einreihen *staged* nur in die offene Arbeitseinheit (`OutboxPublisher::publish`, infallibel) — die
/// fehlbare Schreibung passiert erst am **Commit-Rand** (`UnitOfWork::commit`, siehe `run_in_unit`),
/// wo ein Fehlschlag io-intern als Commit-/Retry-Fall erscheint, nicht als terminaler
/// Publish-Fehler. Darum überall `Ok(())`; der **Vertrag** bleibt fehlbar, damit ein Transport, der
/// beim Einreihen scheitern kann, seinen Fehler mappen kann, ohne dass der Slice sich ändert.
impl<O: OutboxPublisher> WindowPublishSeam for PublishingPort<'_, O> {
    /// Reiht den `WindowClosed`-Zeiger stromabwärts nach `auswerten` ein.
    fn publish_closed(&self, message: WindowClosedMessage) -> Result<(), WindowPublishUnavailable> {
        self.outbox.publish(OutboxHop { command: message });
        Ok(())
    }

    /// Reiht den **nächsten Fenster-Schritt** ein — die Selbst-Fortsetzung des Pagers.
    fn publish_advance(
        &self,
        message: AdvanceWindowMessage,
    ) -> Result<(), WindowPublishUnavailable> {
        self.outbox.publish(OutboxHop { command: message });
        Ok(())
    }

    /// Reiht den `HistoryComplete`-Abschluss mit der Lauf-Spanne ein.
    fn publish_history_complete(
        &self,
        message: HistoryCompleteMessage,
    ) -> Result<(), WindowPublishUnavailable> {
        self.outbox.publish(OutboxHop { command: message });
        Ok(())
    }
}

/// Hüllt ein adapter-seitiges [`PublishCommand`] in ein io-privates [`OutboxCommand`]: der
/// Dispatch-Key propagiert kompilierzeit-fest (`C::MESSAGE_TYPE`), die Aggregat-Identität als
/// `partition_key`. **Kein serde-Inhalt** — solange kein Kommando eine Nutzlast über die
/// Aggregat-ID hinaus trägt, serialisiert der `OutboxPublisher` einen leeren `content` (der
/// nachgelagerte Consumer liest per `partition_key`).
struct OutboxHop<C: PublishCommand> {
    command: C,
}

impl<C: PublishCommand> OutboxCommand for OutboxHop<C> {
    const MESSAGE_TYPE: &'static str = C::MESSAGE_TYPE;

    fn partition_key(&self) -> &str {
        self.command.partition_key()
    }
}
