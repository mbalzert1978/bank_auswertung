use application_kernel::TypedOutboxHandler;
use einlesen_domain::{ReadError, ReadHandler, ReadTransactions, RunConfiguration};
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

use crate::commands::ReadTransactionsMessage;

/// Die **Consume-Bridge**: der öffentliche Outbox-Seam für `ReadTransactions`-Nachrichten.
/// Deserialisiert den Envelope-Inhalt (JSON, [`outbox_payload`](shared_kernel::outbox_payload)-
/// Konvention) zum **feldlosen Wire-DTO** [`ReadTransactionsMessage`] und baut daraus nur noch
/// den **Auslöser** `ReadTransactions` (leere `RunConfiguration`) — den Batch beschafft der
/// Handler selbst über den Datenquellen-Port
/// ([Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md);
/// die Präsenz-Prüfung liegt seitdem in der [`SourceBridge`](crate::bridges::SourceBridge)
/// hinter dem Quell-Seam, nicht mehr hier). Den terminalen [`ReadError`] des Handlers mappt
/// sie über [`TerminalError`] auf `OutboxError::Warning` (Dead-Letter). Die JSON-Kenntnis lebt
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

    fn dispatch(&self, _request: Self::Request) -> Result<(), Self::Error> {
        self.handler
            .handle(ReadTransactions::new(RunConfiguration))
            .map_err(TerminalError)
    }
}

impl OutboxHandler for ConsumeBridge<'_> {
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        self.handle_outbox(message)
    }
}

/// Lokaler Newtype über die **Fehlersumme des Handlers** ([`ReadError`]: Beschaffung/Präsenz
/// oder Deutung — alles terminal → Dead-Letter). Er existiert nur noch für die Orphan-Regel:
/// weder Domänen- noch Kernel-Fehler sind lokal, dieser Newtype ist es und macht das
/// `impl From<TerminalError> for OutboxError` zulässig.
#[derive(Debug)]
pub(crate) struct TerminalError(ReadError);

impl From<TerminalError> for OutboxError {
    fn from(TerminalError(error): TerminalError) -> Self {
        // Der Grund ist die **redigierte** Domänen-Meldung (Feldname, Fehlerart bzw.
        // io-Diagnose der Beschaffung, nie ein Rohwert) — ein Spaltenverschub leakt so keine
        // IBAN in die persistierte Dead-Letter-Meldung.
        OutboxError::Warning {
            reason: error.to_string(),
        }
    }
}
