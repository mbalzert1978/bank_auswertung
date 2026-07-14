use application_kernel::{Command, PublishCommand};

/// Der Dispatch-Key des ausgehenden Commands — die **eine** Quelle, auf die der Outbox-Prozessor
/// des nachgelagerten `obfuskieren`-Slice seine Consume-Bridge auflöst. Das Domänen-Aggregat
/// `LineAggregate` (in `einlesen_domain`) trägt keinen Dispatch-Key; er lebt hier auf dem DTO.
pub(crate) const MESSAGE_TYPE: &str = "Ingested";

/// Das ausgehende **Publish-Command** der „Eingelesen"-Nachricht: ein reiner **Zeiger** auf die
/// unter dieser `id` abgelegte Roh-Erfassung — nur die ID (Tabelle = Wahrheit, Message = Pointer,
/// [0017](../../../docs/issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)).
/// Der nachgelagerte `obfuskieren`-Consumer liest die Nutzlast **per ID** aus dem einlesen-Speicher,
/// nicht aus der Nachricht.
///
/// Es erfüllt `application_kernel::PublishCommand` (adapter-seitiger Vertrag); **kein
/// `shared_kernel::OutboxCommand`** mehr (ADR-0011): den letzten Outbox-Hop macht `io`. **Kein
/// `Serialize`**: die ID reist als `partition_key` der Outbox-Hülle, die abgeflachten Rohfelder
/// werden nicht über die Message dupliziert. Der Typ ist **öffentlich, aber opak** (private Felder,
/// crate-interner Konstruktor): der `ReadingHandler`-Vertrag nennt ihn als `Publisher`-Nutzlast,
/// gebaut wird er nur hier im Adapter.
#[derive(Debug)]
pub struct IngestedMessage {
    id: String,
}

impl IngestedMessage {
    /// Zeigt auf die abgelegte Roh-Erfassung (übernimmt nur ihre ID).
    pub(crate) fn from_id(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl Command for IngestedMessage {}

impl PublishCommand for IngestedMessage {
    const MESSAGE_TYPE: &'static str = MESSAGE_TYPE;

    fn partition_key(&self) -> &str {
        &self.id
    }
}
