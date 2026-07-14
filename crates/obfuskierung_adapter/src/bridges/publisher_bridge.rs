use application_kernel::Publisher;
use obfuskierung_domain::{BookingId, BookingPublisher};

use crate::commands::ObfuscatedMessage;

/// Die **Publisher-Bridge**: erfüllt den internen Domänen-Port `obfuskierung_domain::BookingPublisher`
/// und setzt die [`BookingId`] auf das adapter-seitige [`ObfuscatedMessage`] um (nur die ID = Zeiger),
/// das sie über den `application_kernel::Publisher` weiterreicht. Die einzige Stelle, an der der
/// Domänen-Port und der `Publisher`-Vertrag zusammentreffen — dadurch kann ein Slice-Test einen
/// Fake-Publisher unterschieben und veröffentlichte Kommandos beobachten.
///
/// **Kein `shared_kernel` mehr** ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
/// Entscheidung D): der Ausgang zielt auf `application_kernel::Publisher`; den letzten Outbox-Hop
/// (`PublishCommand → OutboxCommand`) macht `io`. Der Adapter ist damit restlos outbox-frei. Vorbild:
/// der `IngestedPublisher` des Einlese-Slice.
pub(crate) struct PublisherBridge<'a> {
    pub(crate) publisher: &'a dyn Publisher<ObfuscatedMessage>,
}

impl BookingPublisher for PublisherBridge<'_> {
    fn publish(&self, id: &BookingId) {
        self.publisher
            .publish(ObfuscatedMessage::from_id(id.as_str()));
    }
}
