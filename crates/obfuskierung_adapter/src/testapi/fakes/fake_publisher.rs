use std::cell::RefCell;

use application_kernel::{PublishCommand, Publisher};

use crate::commands::ObfuscatedMessage;

/// Beobachtet veröffentlichte Kommandos an der `application_kernel::Publisher`-Grenze getrennt nach
/// Dispatch-Key (`ObfuscatedMessage::MESSAGE_TYPE`, hier `Obfuscated`) und `partition_key()` (hier die
/// publizierte **ID** — der Zeiger auf die abgelegte Buchung). Steht **an der Stelle, an der später
/// `io` sitzt** (den letzten Outbox-Hop macht dort die echte Verdrahtung). Vorbild: der `FakePublisher`
/// des Einlese-Slice.
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

impl Publisher<ObfuscatedMessage> for FakePublisher {
    fn publish(&self, command: ObfuscatedMessage) {
        self.message_types
            .borrow_mut()
            .push(ObfuscatedMessage::MESSAGE_TYPE.to_string());
        self.partition_keys
            .borrow_mut()
            .push(command.partition_key().to_string());
    }
}
