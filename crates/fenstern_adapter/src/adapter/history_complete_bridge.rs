use fenstern_domain::{HistoryCompletePublisher, RunId, WindowError, WindowSpan};

use crate::abstractions::{PublishUnavailable, WindowPublishSeam};
use crate::models::HistoryCompleteMessage;

/// Die **Publisher-Bridge für den Abschluss**: erfüllt den internen Domänen-Port
/// `fenstern_domain::HistoryCompletePublisher` und setzt die [`WindowSpan`] auf die
/// [`HistoryCompleteMessage`] um, die sie über den öffentlichen [`WindowPublishSeam`] einreiht — in
/// **dieselbe** Outbox wie der `WindowClosed`-Zeiger, denn beide committen atomar zusammen
/// (ADR-0013, Entscheidung B). Ein `Err` des Seams ist **terminal** → `WindowError::PublishFailed`
/// mit durchgereichter io-Diagnose.
pub(crate) struct HistoryCompleteBridge<'a> {
    pub(crate) seam: &'a dyn WindowPublishSeam,
}

impl HistoryCompletePublisher for HistoryCompleteBridge<'_> {
    fn publish(&self, run: &RunId, span: &WindowSpan) -> Result<(), WindowError> {
        self.seam
            .publish_history_complete(HistoryCompleteMessage::from_span(
                span.from().value(),
                span.to().value(),
                run.as_str(),
            ))
            .map_err(
                |PublishUnavailable { message }| WindowError::PublishFailed { reason: message },
            )
    }
}
