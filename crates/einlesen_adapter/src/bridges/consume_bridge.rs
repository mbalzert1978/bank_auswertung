use einlesen_domain::{ReadHandler, ReadTransactions, RunConfiguration};
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

/// Die **Consume-Bridge**: der öffentliche Outbox-Seam für `ReadTransactions`-Nachrichten. Die
/// Nachricht ist ein reiner **Auslöser** — sie trägt **keinen** JSON-Inhalt (darum bindet der
/// Slice kein serde); die Quelle lebt allein im [`partition_key`](OutboxMessage::partition_key)
/// der [`OutboxMessage`]-Hülle (ADR-0008-Envelope-Konvention). Die Bridge baut daraus unmittelbar
/// das Domänen-Kommando [`ReadTransactions`] (leere [`RunConfiguration`]) und dispatcht an den
/// domänen-internen [`ReadHandler`] — der **beschafft** den Batch über den Datenquellen-Port
/// ([Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md);
/// die Präsenz-Prüfung liegt in der [`SourceBridge`](crate::bridges::SourceBridge) hinter dem
/// Quell-Seam, nicht hier). Den terminalen [`ReadError`](einlesen_domain::ReadError) des Handlers
/// mappt sie **inline** auf `OutboxError::Warning` (Vorbild `obfuskieren`s `HandleError`-Mapping):
/// jeder Fehler — Beschaffung/Präsenz/Deutung — ist terminal, das Dead-Letter ist batch-atomar.
/// Der Grund ist stets die **redigierte** Domänen-Meldung (Feldname, Fehlerart bzw. io-Diagnose,
/// nie ein Rohwert) — ein Spaltenverschub leakt so keine IBAN in die Dead-Letter-Meldung. Kein
/// Commit, den besorgt io.
pub(crate) struct ConsumeBridge<'a> {
    pub(crate) handler: ReadHandler<'a>,
}

impl OutboxHandler for ConsumeBridge<'_> {
    fn handle(&self, _message: OutboxMessage) -> Result<(), OutboxError> {
        self.handler
            .handle(ReadTransactions::new(RunConfiguration))
            .map_err(|error| OutboxError::Warning {
                reason: error.to_string(),
            })
    }
}

/// Der Dispatch-Key, unter dem der Prozessor diese Bridge auflöst (die konsumierte Nachricht).
pub(crate) const CONSUMED_MESSAGE_TYPE: &str = "ReadTransactions";
