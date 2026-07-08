use std::cell::RefCell;

use shared_kernel::{OutboxCommand, OutboxPublisher};

/// Beobachtet veröffentlichte Kommandos an der Outbox-Grenze getrennt nach
/// Dispatch-Key (`T::MESSAGE_TYPE`) und `partition_key()`.
#[derive(Default)]
pub(crate) struct FakeOutboxPublisher {
    message_types: RefCell<Vec<String>>,
    partition_keys: RefCell<Vec<String>>,
}

impl FakeOutboxPublisher {
    pub(crate) fn message_types(&self) -> Vec<String> {
        self.message_types.borrow().clone()
    }

    pub(crate) fn partition_keys(&self) -> Vec<String> {
        self.partition_keys.borrow().clone()
    }
}

impl OutboxPublisher for FakeOutboxPublisher {
    fn publish<T: OutboxCommand>(&self, command: T) {
        self.message_types
            .borrow_mut()
            .push(T::MESSAGE_TYPE.to_string());
        self.partition_keys
            .borrow_mut()
            .push(command.partition_key().to_string());
    }
}
