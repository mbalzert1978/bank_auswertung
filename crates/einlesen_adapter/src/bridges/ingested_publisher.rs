use application_kernel::Publisher;
use einlesen_domain::{EventSink, IngestionId};

use crate::commands::IngestedMessage;

/// Die **Publisher-Bridge**: erfüllt den internen Domänen-Port `einlesen_domain::EventSink`
/// und setzt die [`IngestionId`] auf das adapter-seitige [`IngestedMessage`] um (nur die ID =
/// Zeiger), das sie über den `application_kernel::Publisher` weiterreicht. Die einzige Stelle,
/// an der der Domänen-Port und der `Publisher`-Vertrag zusammentreffen — dadurch kann ein
/// Slice-Test einen Fake-Publisher unterschieben und veröffentlichte Kommandos beobachten.
///
/// **Kein `shared_kernel` mehr** (ADR-0011, Entscheidung D): der Ausgang zielt auf
/// `application_kernel::Publisher`; den letzten Outbox-Hop (`PublishCommand → OutboxCommand`)
/// macht `io`. Der Adapter ist damit restlos outbox-frei.
pub(crate) struct IngestedPublisher<'a> {
    pub(crate) publisher: &'a dyn Publisher<IngestedMessage>,
}

impl EventSink for IngestedPublisher<'_> {
    fn publish(&self, id: &IngestionId) {
        self.publisher
            .publish(IngestedMessage::from_id(id.as_str()));
    }
}
