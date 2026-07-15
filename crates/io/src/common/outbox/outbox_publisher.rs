use super::OutboxCommand;

/// Handler-seitiger Port, um ein [`OutboxCommand`] in die Outbox einzureihen.
/// **Add-only** (ISP): Lesen und Markieren von Nachrichten ist Sache des
/// Prozessors, nicht dieses Ports. Der Dispatch-Key kommt aus dem Kommando selbst
/// ([`OutboxCommand::MESSAGE_TYPE`]) — keine aufruferseitige Typangabe.
pub trait OutboxPublisher {
    /// Reiht das Kommando in die Outbox ein.
    fn publish<T: OutboxCommand>(&self, command: T);
}
