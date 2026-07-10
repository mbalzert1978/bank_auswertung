use obfuskierung_domain::{BookingId, BookingPublisher};
use shared_kernel::OutboxPublisher;

use crate::commands::ObfuscatedMessage;

/// Die **Publisher-Bridge**: erfüllt `obfuskierung_domain::BookingPublisher` und setzt die
/// [`BookingId`] auf das serialisierbare Wire-DTO [`ObfuscatedMessage`] um (nur die ID = Zeiger),
/// das sie in die Outbox einreiht. Die einzige Stelle, an der der Domänen-Port und
/// `shared_kernel::OutboxPublisher` zusammentreffen — so kann ein Slice-Test einen
/// Fake-`OutboxPublisher` unterschieben. Generisch über den konkreten `OutboxPublisher`, weil
/// `publish<T>` generisch (nicht objekt-sicher) ist.
pub(crate) struct PublisherBridge<'a, O: OutboxPublisher> {
    pub(crate) outbox: &'a O,
}

impl<O: OutboxPublisher> BookingPublisher for PublisherBridge<'_, O> {
    fn publish(&self, id: &BookingId) {
        self.outbox.publish(ObfuscatedMessage::from_id(id.as_str()));
    }
}
