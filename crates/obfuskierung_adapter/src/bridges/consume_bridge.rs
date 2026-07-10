use obfuskierung_domain::{BookingId, HandleError, ObfuscateBooking, ObfuscationHandler};
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

/// Die **Consume-Bridge**: der öffentliche Outbox-Seam für `Ingested`-Nachrichten. Die Nachricht ist
/// ein reiner **Zeiger** — ihre Aggregat-ID liegt in der [`OutboxMessage`]-Hülle
/// ([`id`](OutboxMessage::id)), **kein** JSON-Inhalt (darum bindet der Slice kein serde). Die Bridge
/// baut daraus das Domänen-Kommando [`ObfuscateBooking`] und dispatcht an den domänen-internen
/// [`ObfuscationHandler`] — der **liest** das Aggregat per ID aus dem einlesen-Speicher, obfuskiert,
/// legt ab und publiziert die ID. Sie **trennt die beiden Fehlerklassen**: (noch) **abwesend** →
/// `Retry` (die einlesen-Schreibung kann nachhängen), **ungültig** (Pflichtfeld fehlt) → `Warning`
/// (Dead-Letter, redigierter Grund) — sonst wäre ein ungültiger Datensatz eine Poison-Message
/// (endloses Retry). Der Grund ist stets die redigierte Meldung (opake ID bzw. Feldname, nie ein
/// Rohwert); kein Commit, den besorgt io.
pub(crate) struct ConsumeBridge<'a> {
    pub(crate) handler: ObfuscationHandler<'a>,
}

impl OutboxHandler for ConsumeBridge<'_> {
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        let command = ObfuscateBooking::new(BookingId::new(message.id));
        self.handler.handle(command).map_err(|error| {
            let reason = error.to_string();
            match error {
                // (Noch) kein Aggregat → transient: die Nachricht wird erneut zugestellt.
                HandleError::NotYetAvailable(_) => OutboxError::Retry { reason },
                // Aggregat da, aber ungültig → terminal: Dead-Letter statt Poison-Message.
                HandleError::MalformedSource { .. } => OutboxError::Warning { reason },
            }
        })
    }
}

/// Der Dispatch-Key, unter dem der Prozessor diese Bridge auflöst (die konsumierte Nachricht).
pub(crate) const CONSUMED_MESSAGE_TYPE: &str = "Ingested";
