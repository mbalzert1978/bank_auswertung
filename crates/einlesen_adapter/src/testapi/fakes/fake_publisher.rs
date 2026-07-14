use std::cell::RefCell;

use application_kernel::{PublishCommand, Publisher};

use crate::commands::IngestedMessage;

/// Beobachtet veröffentlichte Kommandos an der `application_kernel::Publisher`-Grenze getrennt
/// nach Dispatch-Key (`IngestedMessage::MESSAGE_TYPE`) und `partition_key()` — die zwei
/// Datenrückgaben, die ein Spec gegen die gestageten Speicher-Schlüssel prüft. Steht **an der
/// Stelle, an der später `io` sitzt** (den letzten Outbox-Hop macht dort die echte Verdrahtung).
#[derive(Default)]
pub(crate) struct FakePublisher {
    message_types: RefCell<Vec<String>>,
    partition_keys: RefCell<Vec<String>>,
}

impl FakePublisher {
    pub(crate) fn message_types(&self) -> Vec<String> {
        self.message_types.borrow().clone()
    }

    pub(crate) fn partition_keys(&self) -> Vec<String> {
        self.partition_keys.borrow().clone()
    }
}

impl Publisher<IngestedMessage> for FakePublisher {
    fn publish(&self, command: IngestedMessage) {
        self.message_types
            .borrow_mut()
            .push(IngestedMessage::MESSAGE_TYPE.to_string());
        self.partition_keys
            .borrow_mut()
            .push(command.partition_key().to_string());
    }
}
