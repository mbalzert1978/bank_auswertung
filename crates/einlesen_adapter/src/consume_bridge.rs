use application_kernel::{Handler, TypedOutboxHandler};
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

use crate::handler::{EinleseError, EinleseHandler};
use crate::umsaetze_einlesen::UmsätzeEinlesen;

/// Die **Consume-Bridge**: der öffentliche Outbox-Seam für `UmsätzeEinlesen`-Nachrichten.
/// Sie deserialisiert den Envelope-Inhalt zum internen Kommando, dispatcht ihn an den
/// [`EinleseHandler`] und mappt dessen Fehler über `Into<OutboxError>` zurück — transient →
/// `Retry`, terminaler Aggregat-Fehler → `Warning` (Dead-Letter), beides die
/// Disposition von [`EinleseError`]. Die eigentliche Verdrahtung (Envelope → Request →
/// `Result`, Leer-Policy) erbt sie als Default aus [`TypedOutboxHandler`]; das
/// `impl OutboxHandler` ist der Dreizeiler, den der Prozessor per `message_type` auflöst —
/// kein Commit, den besorgt io.
pub(crate) struct ConsumeBridge<'a> {
    pub(crate) handler: EinleseHandler<'a>,
}

impl TypedOutboxHandler for ConsumeBridge<'_> {
    type Request = UmsätzeEinlesen;
    type Error = EinleseError;

    fn deserialize(&self, content: &str) -> Option<Self::Request> {
        serde_json::from_str(content).ok()
    }

    fn dispatch(&self, request: Self::Request) -> Result<(), Self::Error> {
        self.handler.handle(request)
    }
}

impl OutboxHandler for ConsumeBridge<'_> {
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        self.handle_outbox(message)
    }
}
