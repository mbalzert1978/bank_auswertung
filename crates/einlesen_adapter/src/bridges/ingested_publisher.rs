use einlesen_domain::{EventSink, IngestionId, SinkError};

use crate::abstractions::{PublishSeam, PublishUnavailable};
use crate::commands::IngestedMessage;

/// Die **Publisher-Bridge**: erfüllt den internen Domänen-Port `einlesen_domain::EventSink` und
/// setzt die [`IngestionId`] auf das adapter-seitige [`IngestedMessage`] um (nur die ID = Zeiger),
/// das sie über den öffentlichen [`PublishSeam`] (den io erfüllt) einreiht. Die einzige Stelle, an
/// der der Domänen-Port und der Publish-Seam zusammentreffen — dadurch kann ein Slice-Test einen
/// Fake unterschieben und veröffentlichte Kommandos beobachten. Ein `Err` des Seams ist **terminal**
/// → [`SinkError::PublishFailed`] mit durchgereichter io-Diagnose.
pub(crate) struct IngestedPublisher<'a> {
    pub(crate) seam: &'a dyn PublishSeam,
}

impl EventSink for IngestedPublisher<'_> {
    fn publish(&self, id: &IngestionId) -> Result<(), SinkError> {
        self.seam
            .publish(IngestedMessage::from_id(id.as_str()))
            .map_err(|PublishUnavailable { message }| SinkError::PublishFailed { reason: message })
    }
}
