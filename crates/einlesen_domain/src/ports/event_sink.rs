use crate::IngestionId;

/// Der von der Domäne **verlangte** Port, um pro gelesener Zeile die **ID** des abgelegten
/// Aggregats als „Eingelesen"-Ereignis zu **veröffentlichen** (Fan-out an den nachgelagerten
/// `obfuskieren`-Slice). Die Nachricht ist ein reiner **Zeiger** — nicht die Nutzlast: die liegt
/// unter dieser [`IngestionId`] in der Roh-Erfassung, die der [`Store`](crate::Store) beim Ablegen
/// vergeben hat (Tabelle = Wahrheit, Message = Pointer,
/// [0017](../../../docs/issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)).
/// Wie beim [`Store`](crate::Store) *staged* der Handler nur (reiht das Ereignis in die offene
/// Arbeitseinheit ein); **wann** committet wird, ist io-Sache — darum kein `Result`. Der Port spricht
/// **Domänen-Vokabular** ([`IngestionId`]); die Abbildung auf das Outbox-Wire-DTO ist Adapter-Sache
/// (ACL, „innen → außen"). Vorbild: `IPublisher<T>.Publish(T)` der C#-Referenz.
pub trait EventSink {
    /// Reiht die ID als „Eingelesen"-Zeiger in die offene Arbeitseinheit ein (Staging). Der Handler
    /// reicht dieselbe ID herein, unter der der [`Store`](crate::Store) soeben abgelegt hat.
    fn publish(&self, id: &IngestionId);
}
