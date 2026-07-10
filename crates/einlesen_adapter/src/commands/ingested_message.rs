use shared_kernel::OutboxCommand;

/// Der Dispatch-Key des ausgehenden Commands — die **eine** Quelle, auf die der Outbox-Prozessor
/// des nachgelagerten `obfuskieren`-Slice seine Consume-Bridge auflöst. Das Domänen-Aggregat
/// `LineAggregate` (in `einlesen_domain`) trägt keinen Dispatch-Key; er lebt hier auf dem Wire-DTO.
pub(crate) const MESSAGE_TYPE: &str = "Ingested";

/// Das ausgehende **Wire-Command** der „Eingelesen"-Nachricht: ein reiner **Zeiger** auf die unter
/// dieser `id` abgelegte Roh-Erfassung — nur die ID, als `partition_key` der Nachricht (Tabelle =
/// Wahrheit, Message = Pointer,
/// [0017](../../../docs/issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)).
/// Der nachgelagerte `obfuskieren`-Consumer liest die Nutzlast **per ID** aus dem einlesen-Speicher,
/// nicht aus der Nachricht. **Kein `Serialize`**: `OutboxCommand` verlangt keins, und die ID reist in
/// der Outbox-Hülle; die abgeflachten Rohfelder werden nicht mehr über die Message dupliziert. Vorbild:
/// `ObfuscatedMessage` des Obfuskierungs-Slice.
#[derive(Debug)]
pub(crate) struct IngestedMessage {
    id: String,
}

impl IngestedMessage {
    /// Zeigt auf die abgelegte Roh-Erfassung (übernimmt nur ihre ID).
    pub(crate) fn from_id(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl OutboxCommand for IngestedMessage {
    const MESSAGE_TYPE: &'static str = MESSAGE_TYPE;

    fn partition_key(&self) -> &str {
        &self.id
    }
}
