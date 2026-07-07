use std::fmt;

use application_kernel::{Handler, TypedOutboxHandler};
use einlesen_domain::ZeilenError;
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

use crate::handler::EinleseHandler;
use crate::umsaetze_einlesen::UmsätzeEinlesen;

/// Die **Consume-Bridge**: der öffentliche Outbox-Seam für `UmsätzeEinlesen`-Nachrichten.
/// Deserialisiert den Envelope-Inhalt (JSON, [`outbox_payload`](shared_kernel::outbox_payload)-
/// Konvention) zum internen Kommando, dispatcht ihn an den [`EinleseHandler`] und mappt dessen
/// terminalen `ZeilenError` über [`TerminalerFehler`] auf `OutboxError::Warning` (Dead-Letter).
/// Die JSON-Kenntnis lebt bewusst **hier** (Adapter-Bridge), nicht im Handler — der bleibt rein
/// domänen-typisiert (`EinleseHandler::Error = ZeilenError`). Die eigentliche Verdrahtung
/// (Envelope → Request → `Result`, Leer-Policy) erbt sie als Default aus
/// [`TypedOutboxHandler`]; das `impl OutboxHandler` ist der Dreizeiler, den der Prozessor per
/// `message_type` auflöst — kein Commit, den besorgt io.
pub(crate) struct ConsumeBridge<'a> {
    pub(crate) handler: EinleseHandler<'a>,
}

impl TypedOutboxHandler for ConsumeBridge<'_> {
    type Request = UmsätzeEinlesen;
    type Error = TerminalerFehler;

    fn deserialize(&self, content: &str) -> Option<Self::Request> {
        serde_json::from_str(content).ok()
    }

    fn dispatch(&self, request: Self::Request) -> Result<(), Self::Error> {
        self.handler.handle(request).map_err(TerminalerFehler)
    }
}

impl OutboxHandler for ConsumeBridge<'_> {
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        self.handle_outbox(message)
    }
}

/// Lokaler Fehler-Wrapper um den terminalen `ZeilenError` (Orphan-Regel: weder `ZeilenError`
/// (Domäne) noch `OutboxError` (Kernel) sind lokal — dieser Newtype ist es, und macht
/// `impl From<...>` in beide Richtungen zulässig). Jeder Aggregat-Fehler ist terminal →
/// Dead-Letter.
#[derive(Debug)]
pub(crate) struct TerminalerFehler(ZeilenError);

impl fmt::Display for TerminalerFehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TerminalerFehler {}

impl From<TerminalerFehler> for OutboxError {
    fn from(fehler: TerminalerFehler) -> Self {
        // Der Grund ist der **redigierte** ZeilenError (kein Rohwert) — ein Spaltenverschub
        // leakt so keine IBAN in die persistierte Dead-Letter-Meldung.
        OutboxError::Warning {
            reason: fehler.0.to_string(),
        }
    }
}
