use crate::{IngestionId, SinkError};

/// Der von der Domäne **verlangte** Port, um pro gelesener Zeile die **ID** des abgelegten
/// Aggregats als „Eingelesen"-Ereignis zu **veröffentlichen** (Fan-out an den nachgelagerten
/// `obfuskieren`-Slice). Die Nachricht ist ein reiner **Zeiger**: die Nutzlast liegt unter dieser
/// [`IngestionId`] in der Roh-Erfassung, die der [`Store`](crate::Store) beim Ablegen vergeben
/// hat (Tabelle = Wahrheit, Message = Pointer). Der Port spricht **Domänen-Vokabular**
/// ([`IngestionId`]); die Abbildung auf das Outbox-Wire-DTO ist Adapter-Sache (ACL,
/// „innen → außen").
///
/// Er ist **fehlbar** und **terminal**: ein misslungenes Einreihen ist ein
/// [`SinkError::PublishFailed`], den der Handler per `?` propagiert.
pub trait EventSink {
    /// Veröffentlicht die ID als „Eingelesen"-Zeiger; `Err` = das Einreihen scheiterte terminal.
    /// Der Handler reicht dieselbe ID herein, unter der der [`Store`](crate::Store) soeben abgelegt hat.
    fn publish(&self, id: &IngestionId) -> Result<(), SinkError>;
}
