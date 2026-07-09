use std::fmt;

use application_kernel::TypedOutboxHandler;
use einlesen_domain::{LineError, ReadHandler, ReadTransactions};
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

use crate::commands::ReadTransactionsMessage;
use crate::dto::RawRecord;

/// Die **Consume-Bridge**: der öffentliche Outbox-Seam für `ReadTransactions`-Nachrichten.
/// Deserialisiert den Envelope-Inhalt (JSON, [`outbox_payload`](shared_kernel::outbox_payload)-
/// Konvention) zum **Wire-DTO** [`ReadTransactionsMessage`], mappt es **„außen → innen"** auf das
/// Domänen-Kommando `ReadTransactions` (jede `RawRecord`-Zelle → Domänen-VO `RawLine`),
/// dispatcht an den domänen-internen [`ReadHandler`] und mappt dessen terminalen `LineError`
/// über [`TerminalError`] auf `OutboxError::Warning` (Dead-Letter). Die JSON-Kenntnis lebt
/// bewusst **hier** (Adapter-Bridge), nicht im Handler — der bleibt rein domänen-typisiert. Die
/// eigentliche Verdrahtung (Envelope → Request → `Result`, Leer-Policy) erbt sie als Default aus
/// [`TypedOutboxHandler`]; das `impl OutboxHandler` ist der Dreizeiler, den der Prozessor per
/// `message_type` auflöst — kein Commit, den besorgt io.
pub(crate) struct ConsumeBridge<'a> {
    pub(crate) handler: ReadHandler<'a>,
}

impl TypedOutboxHandler for ConsumeBridge<'_> {
    type Request = ReadTransactionsMessage;
    type Error = TerminalError;

    fn deserialize(&self, content: &str) -> Option<Self::Request> {
        serde_json::from_str(content).ok()
    }

    fn dispatch(&self, request: Self::Request) -> Result<(), Self::Error> {
        let lines = request.lines.iter().map(RawRecord::to_raw_line).collect();
        self.handler
            .handle(ReadTransactions::new(lines))
            .map_err(TerminalError)
    }
}

impl OutboxHandler for ConsumeBridge<'_> {
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        self.handle_outbox(message)
    }
}

/// Lokaler Fehler-Wrapper um den terminalen `LineError` (Orphan-Regel: weder `LineError`
/// (Domäne) noch `OutboxError` (Kernel) sind lokal — dieser Newtype ist es, und macht
/// `impl From<...>` in beide Richtungen zulässig). Jeder Aggregat-Fehler ist terminal →
/// Dead-Letter.
#[derive(Debug)]
pub(crate) struct TerminalError(LineError);

impl fmt::Display for TerminalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TerminalError {}

impl From<TerminalError> for OutboxError {
    fn from(error: TerminalError) -> Self {
        // Der Grund ist der **redigierte** LineError (kein Rohwert) — ein Spaltenverschub
        // leakt so keine IBAN in die persistierte Dead-Letter-Meldung.
        OutboxError::Warning {
            reason: error.0.to_string(),
        }
    }
}
