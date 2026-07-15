use std::cell::RefCell;

use application_kernel::PublishCommand;

use crate::abstractions::{PublishSeam, PublishUnavailable};
use crate::commands::IngestedMessage;

/// Beobachtet veröffentlichte Kommandos am [`PublishSeam`] getrennt nach Dispatch-Key
/// (`IngestedMessage::MESSAGE_TYPE`) und `partition_key()` — die zwei Datenrückgaben, die ein Spec
/// gegen die gestageten Speicher-Schlüssel prüft. Steht **an der Stelle, an der später `io` sitzt**
/// (den letzten Outbox-Hop macht dort die echte Verdrahtung). Ein arrangierter Ausfall
/// (`fail_with`) lässt `publish` terminal scheitern.
#[derive(Default)]
pub(crate) struct FakePublisher {
    message_types: RefCell<Vec<String>>,
    partition_keys: RefCell<Vec<String>>,
    unavailable: Option<String>,
}

impl FakePublisher {
    pub(crate) fn message_types(&self) -> Vec<String> {
        self.message_types.borrow().clone()
    }

    pub(crate) fn partition_keys(&self) -> Vec<String> {
        self.partition_keys.borrow().clone()
    }

    /// Arrangiert den Fehlschlag der Veröffentlichung mit der gegebenen io-Diagnose.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.unavailable = Some(message.to_string());
    }
}

impl PublishSeam for FakePublisher {
    fn publish(&self, message: IngestedMessage) -> Result<(), PublishUnavailable> {
        if let Some(diagnose) = &self.unavailable {
            return Err(PublishUnavailable {
                message: diagnose.clone(),
            });
        }
        self.message_types
            .borrow_mut()
            .push(IngestedMessage::MESSAGE_TYPE.to_string());
        self.partition_keys
            .borrow_mut()
            .push(message.partition_key().to_string());
        Ok(())
    }
}
