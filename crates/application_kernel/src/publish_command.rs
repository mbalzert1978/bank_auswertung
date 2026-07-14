use crate::Command;

/// Ein Kommando, das über einen [`Publisher`](crate::Publisher) zur späteren
/// Verarbeitung versendet werden kann — der **adapter-seitige** Zwilling des io-privaten
/// `shared_kernel::OutboxCommand`. Der Slice-Adapter spricht nur dieses Vokabular; `io`
/// macht den letzten Outbox-Hop und bildet einen `PublishCommand` auf ein `OutboxCommand`
/// ab ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
/// Entscheidung D). Darum trägt dieser Vertrag genau das, was `io` für die Hülle braucht:
/// den Dispatch-Key ([`MESSAGE_TYPE`](PublishCommand::MESSAGE_TYPE)) und die
/// Aggregat-Identität ([`partition_key`](PublishCommand::partition_key)) — **keinen** serde-
/// Inhalt, solange kein Kommando eine echte Nutzlast über die Aggregat-ID hinaus trägt
/// (heute: einlesen ist ein reiner Zeiger).
pub trait PublishCommand: Command {
    /// Der Dispatch-Key dieses Kommandotyps. **Assoziierte Konstante statt
    /// Methode:** der Key ist eine kompilierzeit-feste Eigenschaft des Typs
    /// (`C::MESSAGE_TYPE`), kein Verhalten — die idiomatische Rust-Form dafür.
    const MESSAGE_TYPE: &'static str;

    /// Die Identität des Aggregats, auf das dieses Kommando zeigt. **Instanz-abhängig**
    /// (Methode statt Konstante), weil sie vom konkreten Kommando abhängt; `io` reicht sie
    /// beim Outbox-Hop als `partition_key` der Nachricht durch, auf dem der
    /// Verarbeitungszyklus serialisiert.
    fn partition_key(&self) -> &str;
}
