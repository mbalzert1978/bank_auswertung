use shared_kernel::OutboxCommand;

/// Der Dispatch-Key der ausgehenden Nachricht — die eine Quelle, auf die der Outbox-Prozessor des
/// nachgelagerten `auswerten`-Slice seine Consume-Bridge auflöst.
pub(crate) const MESSAGE_TYPE: &str = "Obfuscated";

/// Das ausgehende **Wire-Command** der `Obfuscated`-Nachricht: ein reiner **Zeiger** auf die in der
/// `obfuskiert`-Tabelle abgelegte Buchung — nur die `id`, als `partition_key` der Nachricht (Tabelle
/// = Wahrheit, Message = Pointer). **Kein `Serialize`**: `OutboxCommand` verlangt keins, und die ID
/// reist in der Outbox-Hülle; die spätere io-Serialisierung des Inhalts (falls nötig) kommt mit io.
#[derive(Debug)]
pub(crate) struct ObfuscatedMessage {
    id: String,
}

impl ObfuscatedMessage {
    /// Zeigt auf die abgelegte Buchung (übernimmt nur ihre ID).
    pub(crate) fn from_id(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl OutboxCommand for ObfuscatedMessage {
    const MESSAGE_TYPE: &'static str = MESSAGE_TYPE;

    fn partition_key(&self) -> &str {
        &self.id
    }
}
