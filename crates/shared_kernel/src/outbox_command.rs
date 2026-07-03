/// Ein Kommando, das in die Outbox veröffentlicht werden kann (Wire-Marker,
/// getrennt vom Domänen-`PublishCommand` im `application_kernel`: die
/// Publisher-Bridge mappt das Domänen-Kommando auf dieses Wire-DTO). Jeder
/// Kommandotyp trägt seinen eigenen Dispatch-Key über
/// [`MESSAGE_TYPE`](OutboxCommand::MESSAGE_TYPE) — die eine Quelle, die Publisher,
/// Prozessor-Dispatch und
/// [`OutboxMessage::message_type`](crate::OutboxMessage::message_type) teilen.
pub trait OutboxCommand {
    /// Der Dispatch-Key dieses Kommandotyps. **Assoziierte Konstante statt
    /// Methode:** ein kompilierzeit-fester, typgebundener Wert (`T::MESSAGE_TYPE`)
    /// — kein Verhalten. Der instanzabhängige
    /// [`partition_key`](OutboxCommand::partition_key) bleibt daneben eine Methode,
    /// weil er vom konkreten Kommando abhängt.
    const MESSAGE_TYPE: &'static str;

    /// Identität des Aggregats, auf das dieses Kommando wirkt. Der Outbox-Zyklus
    /// verarbeitet Kommandos mit gleichem `partition_key` **seriell** (in
    /// Ereignisreihenfolge) und mit verschiedenen Keys **nebenläufig** — zwei
    /// Kommandos auf demselben Aggregat laufen nie gleichzeitig.
    fn partition_key(&self) -> &str;
}
