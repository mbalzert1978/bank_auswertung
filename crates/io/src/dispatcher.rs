use std::collections::HashMap;

use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

/// Der **Broker-Dispatch** ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):
/// löst eine Nachricht über ihren [`message_type`](shared_kernel::OutboxMessage::message_type) auf
/// den registrierten Consumer auf und übergibt sie. Ein [`OutboxConsumer`](crate::OutboxConsumer)
/// wird als `dyn OutboxHandler` registriert; so hält der Dispatcher heterogene Slice-Consumer
/// uniform. Heute ist nur `einlesen` verdrahtet (`obfuskieren` folgt, wenn es migriert ist —
/// bis dahin bleibt es bewusst auf seiner eigenen Consume-Bridge, ADR-0011/Issue 0020).
///
/// Der Lebensdauer-Parameter `'a` bindet die registrierten Consumer an die Seams, die sie borgen
/// (heute die Test-Fakes; später die realen io-Adapter).
#[derive(Default)]
pub struct Dispatcher<'a> {
    consumers: HashMap<String, Box<dyn OutboxHandler + 'a>>,
}

impl<'a> Dispatcher<'a> {
    /// Ein leerer Dispatcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registriert den Consumer für einen `message_type` und gibt einen **verdrängten** Consumer
    /// zurück, falls der Key schon belegt war (`Some` ⇒ Doppel-Registrierung — ein Verdrahtungs-
    /// fehler, der so **nicht** stillschweigend verschluckt wird; im Normalfall `None`, eine Quelle
    /// je Dispatch-Key).
    #[must_use = "ein zurückgegebener Consumer signalisiert eine doppelte Registrierung"]
    pub fn register(
        &mut self,
        message_type: &str,
        consumer: Box<dyn OutboxHandler + 'a>,
    ) -> Option<Box<dyn OutboxHandler + 'a>> {
        self.consumers.insert(message_type.to_string(), consumer)
    }

    /// Löst die Nachricht per `message_type` auf und übergibt sie ihrem Consumer. Ohne passenden
    /// Consumer ist die Nachricht auf der Dispatch-Ebene unzustellbar → `Critical`.
    pub fn dispatch(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        match self.consumers.get(&message.message_type) {
            Some(consumer) => consumer.handle(message),
            None => Err(OutboxError::Critical {
                message: format!(
                    "kein Consumer für message_type {} registriert",
                    message.message_type
                ),
                source: message.message_type,
            }),
        }
    }
}
