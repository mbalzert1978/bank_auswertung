use einlesen_domain::{EventSink, IngestionId};
use shared_kernel::OutboxPublisher;

use crate::commands::IngestedMessage;

/// Die **Publisher-Bridge**: erfüllt den internen Domänen-Port `einlesen_domain::EventSink`
/// und setzt die [`IngestionId`] auf das Wire-DTO [`IngestedMessage`] um (nur die ID = Zeiger),
/// das sie in die Outbox einreiht. Die einzige Stelle, an der der Domänen-Port und
/// `shared_kernel::OutboxPublisher` zusammentreffen — dadurch kann ein Slice-Test einen
/// Fake-`OutboxPublisher` unterschieben und veröffentlichte Kommandos beobachten.
///
/// Generisch über den konkreten `OutboxPublisher`, weil dessen `publish<T>` eine generische
/// Methode ist (nicht objekt-sicher) — kein `&dyn OutboxPublisher` möglich.
pub(crate) struct IngestedPublisher<'a, O: OutboxPublisher> {
    pub(crate) outbox: &'a O,
}

impl<O: OutboxPublisher> EventSink for IngestedPublisher<'_, O> {
    fn publish(&self, id: &IngestionId) {
        self.outbox.publish(IngestedMessage::from_id(id.as_str()));
    }
}
