use crate::{OutboxError, OutboxMessage};

/// Öffentlicher Consume-Seam: der Outbox-Prozessor dispatcht **eine** Nachricht an
/// den Handler, aufgelöst über
/// [`OutboxMessage::message_type`](crate::OutboxMessage::message_type). Seit dem
/// Kernel-Flip ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md))
/// implementiert **`io`s generische `OutboxConsumer`** diesen Seam **einmal** über
/// **jeden** Slice (statt je Slice eine eigene Consume-Bridge): sie extrahiert den
/// Envelope, ruft den slice-gelieferten `CommandDecoder`, dispatcht an den
/// Domänen-Handler und mappt dessen Fehler auf ein [`OutboxError`]: `Ok(())` markiert
/// die Nachricht als verarbeitet, `Err` steuert Retry bzw. Dead-Letter.
///
/// **Bleibt trotz Rusts `Result` ein eigener Trait** (anders als der TODO-Verdacht
/// „vermutlich überflüssig"): er ist der **objekt-sichere** Dispatch-Seam, über den
/// der Prozessor heterogene Handler uniform hält (`dyn OutboxHandler`) und per
/// `message_type` auflöst. Der generische `application_kernel::Handler` kann das wegen
/// seiner assoziierten Typen (`Command`/`Error`) nicht — er ist nicht objekt-sicher;
/// dieser Trait ist die uniforme Fassade davor.
pub trait OutboxHandler {
    /// Verarbeitet genau eine Outbox-Nachricht: `Ok(())` = verarbeitet, `Err` =
    /// Retry/Dead-Letter (s. [`OutboxError`]).
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError>;
}
