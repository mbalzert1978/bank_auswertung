use std::fmt;

use application_kernel::TypedOutboxHandler;
use einlesen_domain::{LineError, ReadHandler, ReadTransactions};
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

use crate::commands::ReadTransactionsMessage;
use crate::dto::{MissingMandatoryField, RawRecord};

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
        // Erst **den ganzen Batch** „außen → innen" abbilden: fehlt *irgendwo* ein Pflichtfeld,
        // scheitert das `collect` und der **ganze** Batch dead-lettert (batch-atomar), bevor der
        // Handler eine einzige Zeile deutet.
        let lines = request
            .lines
            .iter()
            .map(RawRecord::to_raw_line)
            .collect::<Result<Vec<_>, _>>()
            .map_err(TerminalError::MissingField)?;
        self.handler
            .handle(ReadTransactions::new(lines))
            .map_err(TerminalError::Line)
    }
}

impl OutboxHandler for ConsumeBridge<'_> {
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        self.handle_outbox(message)
    }
}

/// Lokaler Fehler-Wrapper über die **zwei** terminalen Quellen des Slice: ein fehlendes
/// Pflichtfeld der ACL-Abbildung ([`MissingMandatoryField`]) und ein Deutungs-Fehler der
/// Domäne (`LineError`). Beide sind terminal → Dead-Letter. Der lokale Typ trägt zugleich die
/// Orphan-Regel: weder Domänen- noch Kernel-Fehler sind lokal, dieser Newtype ist es und macht
/// das `impl From<TerminalError> for OutboxError` zulässig.
#[derive(Debug)]
pub(crate) enum TerminalError {
    /// Ein Pflichtfeld fehlte in der Rohzeile (Adapter-ACL, vor dem Deuten).
    MissingField(MissingMandatoryField),
    /// Ein vorhandener Wert war nicht deutbar (Domäne).
    Line(LineError),
}

impl fmt::Display for TerminalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TerminalError::MissingField(error) => write!(f, "{error}"),
            TerminalError::Line(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for TerminalError {}

impl From<TerminalError> for OutboxError {
    fn from(error: TerminalError) -> Self {
        // Der Grund ist die **redigierte** Meldung (Feldname bzw. Fehlerart, nie ein Rohwert)
        // — ein Spaltenverschub leakt so keine IBAN in die persistierte Dead-Letter-Meldung.
        OutboxError::Warning {
            reason: error.to_string(),
        }
    }
}
