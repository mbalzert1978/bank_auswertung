use application_kernel::Publisher;
use obfuskierung_domain::{BookingId, BookingPublisher, ObfuscationError};

use crate::models::ObfuscatedMessage;

/// Die **Publisher-Bridge**: erfüllt den internen Domänen-Port `obfuskierung_domain::BookingPublisher`
/// und setzt die [`BookingId`] auf das adapter-seitige [`ObfuscatedMessage`] um (nur die ID = Zeiger),
/// das sie über den `application_kernel::Publisher` weiterreicht. Die einzige Stelle, an der der
/// Domänen-Port und der `Publisher`-Vertrag zusammentreffen — dadurch kann ein Slice-Test einen
/// Fake-Publisher unterschieben und veröffentlichte Kommandos beobachten. Der Ausgang zielt auf
/// `application_kernel::Publisher`; den letzten Outbox-Hop macht `io`.
pub(crate) struct PublisherBridge<'a> {
    pub(crate) publisher: &'a dyn Publisher<ObfuscatedMessage>,
}

impl BookingPublisher for PublisherBridge<'_> {
    fn publish(&self, id: &BookingId) -> Result<(), ObfuscationError> {
        self.publisher
            .publish(ObfuscatedMessage::from_id(id.as_str()));
        Ok(())
    }
}
