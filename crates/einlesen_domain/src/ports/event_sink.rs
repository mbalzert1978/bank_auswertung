use crate::LineAggregate;

/// Der von der Domäne **verlangte** Port, um pro gelesener Zeile das validierte
/// [`LineAggregate`](crate::LineAggregate) als „Eingelesen"-Ereignis zu **veröffentlichen**
/// (Fan-out an den nachgelagerten `obfuskieren`-Slice). Wie beim [`Store`](crate::Store)
/// *staged* der Handler nur (reiht das Ereignis in die offene Arbeitseinheit ein); **wann**
/// committet wird, ist io-Sache — darum `void`, kein `Result`. Der Port spricht
/// **Domänen-Vokabular** (`LineAggregate`); die Abbildung auf das Outbox-Wire-DTO ist
/// Adapter-Sache (ACL, „innen → außen: Domänen-VO → DTO"). Vorbild: `IPublisher<T>.Publish(T)`
/// der C#-Referenz.
pub trait EventSink {
    /// Reiht das Aggregat als „Eingelesen"-Ereignis in die offene Arbeitseinheit ein (Staging).
    fn publish(&self, aggregate: LineAggregate);
}
