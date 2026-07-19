use fenstern_domain::{WindowClosedPublisher, WindowError, WindowId};

use crate::abstractions::{PublishUnavailable, WindowPublishSeam};
use crate::models::WindowClosedMessage;

/// Die **Publisher-Bridge stromabwärts**: erfüllt den internen Domänen-Port
/// `fenstern_domain::WindowClosedPublisher` und setzt die [`WindowId`] auf die
/// [`WindowClosedMessage`] um (nur die ID = Zeiger), die sie über den öffentlichen
/// [`WindowPublishSeam`] einreiht. Die einzige Stelle, an der der Domänen-Port und der Publish-Seam
/// zusammentreffen — dadurch kann ein Slice-Test einen Fake unterschieben und veröffentlichte
/// Kommandos beobachten. Ein `Err` des Seams ist **terminal** → `WindowError::PublishFailed` mit
/// durchgereichter io-Diagnose.
pub(crate) struct WindowClosedBridge<'a> {
    pub(crate) seam: &'a dyn WindowPublishSeam,
}

impl WindowClosedPublisher for WindowClosedBridge<'_> {
    fn publish(&self, id: &WindowId) -> Result<(), WindowError> {
        self.seam
            .publish_closed(WindowClosedMessage::from_id(id.as_str()))
            .map_err(
                |PublishUnavailable { message }| WindowError::PublishFailed { reason: message },
            )
    }
}
