use application_kernel::Publisher;
use shared_kernel::OutboxPublisher;

use crate::eingelesen::Eingelesen;
use crate::eingelesen_message::EingelesenMessage;

/// Die **Publisher-Bridge**: setzt das Domänen-Kommando `Eingelesen` auf sein serialisierbares
/// Wire-DTO [`EingelesenMessage`] um und reiht es in die Outbox ein. Die einzige Stelle, an
/// der die zwei Marker (`application_kernel::Publisher` und `shared_kernel::OutboxPublisher`)
/// zusammentreffen — dadurch kann ein Slice-Test einen Fake-`OutboxPublisher` unterschieben
/// und veröffentlichte Kommandos beobachten.
///
/// Generisch über den konkreten `OutboxPublisher`, weil dessen `publish<T>` eine generische
/// Methode ist (nicht objekt-sicher) — kein `&dyn OutboxPublisher` möglich.
pub(crate) struct EingelesenPublisher<'a, O: OutboxPublisher> {
    pub(crate) outbox: &'a O,
}

impl<O: OutboxPublisher> Publisher<Eingelesen> for EingelesenPublisher<'_, O> {
    fn publish(&self, command: Eingelesen) {
        self.outbox
            .publish(EingelesenMessage::aus_zeile(&command.zeile));
    }
}
