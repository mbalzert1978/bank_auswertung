use std::cell::RefCell;

use application_kernel::PublishCommand;

use crate::abstractions::{PublishUnavailable, WindowPublishSeam};
use crate::models::{HistoryCompleteMessage, WindowClosedMessage};

/// Beobachtet die eingereihten Nachrichten am Publish-Seam getrennt nach Dispatch-Key
/// (`MESSAGE_TYPE`) und `partition_key()`. Steht **an der Stelle, an der später `io` sitzt** (den
/// letzten Outbox-Hop macht dort die echte Verdrahtung).
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Enqueue-Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeWindowPublisher {
    message_types: RefCell<Vec<String>>,
    partition_keys: RefCell<Vec<String>>,
    failure: Option<String>,
}

impl FakeWindowPublisher {
    pub(crate) fn message_types(&self) -> Vec<String> {
        self.message_types.borrow().clone()
    }

    pub(crate) fn partition_keys(&self) -> Vec<String> {
        self.partition_keys.borrow().clone()
    }

    /// Lässt jedes Einreihen terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure = Some(message.to_string());
    }

    /// Hält eine eingereihte Nachricht fest — oder spielt den terminalen Fehlschlag.
    fn record(&self, message_type: &str, partition_key: &str) -> Result<(), PublishUnavailable> {
        self.failure
            .clone()
            .map_or(Ok(()), |message| Err(PublishUnavailable { message }))?;
        self.message_types
            .borrow_mut()
            .push(message_type.to_string());
        self.partition_keys
            .borrow_mut()
            .push(partition_key.to_string());
        Ok(())
    }
}

impl WindowPublishSeam for FakeWindowPublisher {
    fn publish_closed(&self, message: WindowClosedMessage) -> Result<(), PublishUnavailable> {
        self.record(WindowClosedMessage::MESSAGE_TYPE, message.partition_key())
    }

    fn publish_history_complete(
        &self,
        message: HistoryCompleteMessage,
    ) -> Result<(), PublishUnavailable> {
        self.record(
            HistoryCompleteMessage::MESSAGE_TYPE,
            message.partition_key(),
        )
    }
}
