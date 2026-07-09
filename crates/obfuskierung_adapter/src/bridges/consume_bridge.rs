use obfuskierung_domain::{BookingId, ObfuscateBooking, ObfuscationHandler};
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

/// Die **Consume-Bridge**: der öffentliche Outbox-Seam für `Ingested`-Nachrichten. Die Nachricht ist
/// ein reiner **Zeiger** — ihre Aggregat-ID liegt in der [`OutboxMessage`]-Hülle
/// ([`id`](OutboxMessage::id)), **kein** JSON-Inhalt (darum bindet der Slice kein serde). Die Bridge
/// baut daraus das Domänen-Kommando [`ObfuscateBooking`] und dispatcht an den domänen-internen
/// [`ObfuscationHandler`] — der **liest** das Aggregat per ID aus dem einlesen-Speicher, obfuskiert,
/// legt ab und publiziert die ID. „Unbekannte ID" mappt auf **Retry** (die einlesen-Schreibung kann
/// nachhängen), nicht auf Dead-Letter; kein Commit, den besorgt io.
pub(crate) struct ConsumeBridge<'a> {
    pub(crate) handler: ObfuscationHandler<'a>,
}

impl OutboxHandler for ConsumeBridge<'_> {
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        let command = ObfuscateBooking::new(BookingId::new(message.id));
        // Zur ID (noch) kein Aggregat → transient: die Nachricht wird erneut zugestellt.
        self.handler
            .handle(command)
            .map_err(|unknown| OutboxError::Retry {
                reason: unknown.to_string(),
            })
    }
}

/// Der Dispatch-Key, unter dem der Prozessor diese Bridge auflöst (die konsumierte Nachricht).
pub(crate) const CONSUMED_MESSAGE_TYPE: &str = "Ingested";
