use crate::{OutboxError, OutboxMessage};

/// Öffentlicher Consume-Seam: der Outbox-Prozessor dispatcht **eine** Nachricht an
/// den Handler, aufgelöst über
/// [`OutboxMessage::message_type`](crate::OutboxMessage::message_type). Die
/// Handler-Bridge eines Slice implementiert diesen Seam (bequem über die
/// `TypedOutboxHandler`-Basis im `application_kernel`), mappt den Envelope auf ihren
/// internen Domänen-Handler und dessen Fehler zurück auf ein [`OutboxError`]:
/// `Ok(())` markiert die Nachricht als verarbeitet, `Err` steuert Retry bzw.
/// Dead-Letter.
///
/// **Bleibt trotz Rusts `Result` ein eigener Trait** (anders als der TODO-Verdacht
/// „vermutlich überflüssig"): er ist der **objekt-sichere** Dispatch-Seam, über den
/// der Prozessor heterogene Handler uniform hält (`dyn OutboxHandler`) und per
/// `message_type` auflöst. Die `TypedOutboxHandler`-Basis (im `application_kernel`)
/// kann das wegen ihrer assoziierten Typen (`Request`/`Error`) nicht — sie ist nicht
/// objekt-sicher; dieser Trait ist die uniforme Fassade davor.
pub trait OutboxHandler {
    /// Verarbeitet genau eine Outbox-Nachricht: `Ok(())` = verarbeitet, `Err` =
    /// Retry/Dead-Letter (s. [`OutboxError`]).
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError>;
}
