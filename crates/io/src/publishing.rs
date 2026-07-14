use application_kernel::{PublishCommand, Publisher};
use shared_kernel::{OutboxCommand, OutboxPublisher};

/// Der **letzte Outbox-Hop** ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
/// Entscheidung D): erfüllt den adapter-seitigen `application_kernel::Publisher` und reicht das
/// veröffentlichte Kommando an den io-privaten `shared_kernel::OutboxPublisher` durch — die eine
/// Stelle, an der `PublishCommand` (adapter-sichtbar) auf `OutboxCommand` (io-privat) trifft. Der
/// Slice-Adapter zielt so auf `application_kernel::Publisher` und berührt die Outbox-Mechanik nie.
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
