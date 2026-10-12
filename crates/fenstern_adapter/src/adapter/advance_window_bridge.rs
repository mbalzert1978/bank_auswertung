use fenstern_domain::{AdvanceWindow, AdvanceWindowPublisher, WindowError};

use crate::abstractions::{PublishUnavailable, WindowPublishSeam};
use crate::models::AdvanceWindowMessage;

/// Die **Publisher-Bridge der Selbst-Fortsetzung**: erfüllt den internen Domänen-Port
/// `fenstern_domain::AdvanceWindowPublisher` und setzt das nächste Fenster-Kommando auf die
/// [`AdvanceWindowMessage`] um, die sie über den öffentlichen [`WindowPublishSeam`] einreiht — in
/// **dieselbe** Outbox wie der `WindowClosed`-Zeiger, denn beide committen atomar zusammen
/// (ADR-0013, Entscheidung B). Ein `Err` des Seams ist **terminal** → `WindowError::PublishFailed`
/// mit durchgereichter io-Diagnose.
pub(crate) struct AdvanceWindowBridge<'a> {
    pub(crate) seam: &'a dyn WindowPublishSeam,
}

impl AdvanceWindowPublisher for AdvanceWindowBridge<'_> {
    fn publish(&self, command: &AdvanceWindow) -> Result<(), WindowError> {
        self.seam
            .publish_advance(AdvanceWindowMessage::from_anchor(
                command.anchor().value(),
                command.run_top().value(),
                command.run().as_str(),
            ))
            .map_err(
                |PublishUnavailable { message }| WindowError::PublishFailed { reason: message },
            )
    }
}
