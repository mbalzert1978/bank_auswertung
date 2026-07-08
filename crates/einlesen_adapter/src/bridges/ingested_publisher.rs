use einlesen_domain::{EventSink, LineAggregate};
use shared_kernel::OutboxPublisher;

use crate::commands::IngestedMessage;
use crate::dto::Line;

/// Die **Publisher-Bridge**: erfüllt den internen Domänen-Port `einlesen_domain::EventSink`
/// und setzt das Domänen-Aggregat `LineAggregate` auf sein serialisierbares Wire-DTO
/// [`IngestedMessage`] um (ACL, „innen → außen: Domänen-VO → DTO"), das sie in die Outbox
/// einreiht. Die einzige Stelle, an der der Domänen-Port und `shared_kernel::OutboxPublisher`
/// zusammentreffen — dadurch kann ein Slice-Test einen Fake-`OutboxPublisher` unterschieben und
/// veröffentlichte Kommandos beobachten.
///
/// Generisch über den konkreten `OutboxPublisher`, weil dessen `publish<T>` eine generische
/// Methode ist (nicht objekt-sicher) — kein `&dyn OutboxPublisher` möglich.
pub(crate) struct IngestedPublisher<'a, O: OutboxPublisher> {
    pub(crate) outbox: &'a O,
}

impl<O: OutboxPublisher> EventSink for IngestedPublisher<'_, O> {
    fn publish(&self, aggregate: LineAggregate) {
        self.outbox
            .publish(IngestedMessage::from_line(&Line::from_aggregate(
                &aggregate,
            )));
    }
}
