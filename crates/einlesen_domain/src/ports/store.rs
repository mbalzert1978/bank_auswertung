use crate::LineAggregate;

/// Der von der Domäne **verlangte** Port, um ein validiertes `LineAggregate` zu
/// **persistieren**. Der Handler *staged* nur (fügt der offenen Arbeitseinheit hinzu) —
/// **wie** und **wann** committet wird, weiß die Domäne nicht: das ist io-Sache (die
/// Outbox-Infrastruktur klammert Persistenz des Aggregats **und** das Einreihen des
/// „Eingelesen"-Ereignisses in **eine** Transaktion,
/// [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)). Darum ist
/// `store` **fehlerfrei** (`void`): das Staging kann nicht fachlich scheitern, der Commit
/// (io) schon. Vorbild: `IFileGroupRepository.Add(group)` der C#-Referenz.
pub trait Store {
    /// Reiht das Aggregat in die offene Arbeitseinheit ein (Staging).
    fn store(&self, aggregate: &LineAggregate);
}
