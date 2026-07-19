use application_kernel::{Command, PublishCommand};

/// Der Dispatch-Key der ausgehenden Nachricht — die eine Quelle, auf die der Outbox-Prozessor des
/// nachgelagerten `auswerten`-Slice seinen Consumer auflöst. Sprach-Konvention: der Code-Name ist
/// `Obfuscated` (deutsch „obfuskiert" bleibt der Prosa-Begriff).
pub(crate) const MESSAGE_TYPE: &str = "Obfuscated";

/// Das ausgehende **Publish-Command** der `Obfuscated`-Nachricht: ein reiner **Zeiger** auf die in der
/// `obfuskiert`-Tabelle abgelegte Buchung — nur die `id`, als `partition_key` der Nachricht (Tabelle
/// = Wahrheit, Message = Pointer).
///
/// Es erfüllt `application_kernel::PublishCommand` (adapter-seitiger Vertrag); **kein
/// io-privates `OutboxCommand`** mehr ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
/// Entscheidung D): den letzten Outbox-Hop macht `io`. **Kein `Serialize`**: `PublishCommand` verlangt
/// keins, die ID reist in der Outbox-Hülle; die spätere io-Serialisierung des Inhalts (falls nötig)
/// kommt mit io. Der Typ ist **öffentlich, aber opak** (private Felder, crate-interner Konstruktor):
/// der [`ObfuscatingHandler`](crate::ObfuscatingHandler)-Vertrag nennt ihn als `Publisher`-Nutzlast,
/// gebaut wird er nur hier im Adapter.
#[derive(Debug)]
pub struct ObfuscatedMessage {
    id: String,
}

impl ObfuscatedMessage {
    /// Zeigt auf die abgelegte Buchung (übernimmt nur ihre ID).
    pub(crate) fn from_id(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl Command for ObfuscatedMessage {}

impl PublishCommand for ObfuscatedMessage {
    const MESSAGE_TYPE: &'static str = MESSAGE_TYPE;

    fn partition_key(&self) -> &str {
        &self.id
    }
}
