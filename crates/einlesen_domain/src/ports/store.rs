use crate::{IngestionId, LineAggregate};

/// Der von der Domäne **verlangte** Port, um ein validiertes `LineAggregate` zu
/// **persistieren**. Der Handler *staged* nur (fügt der offenen Arbeitseinheit hinzu) —
/// **wie** und **wann** committet wird, weiß die Domäne nicht: das ist io-Sache (die
/// Outbox-Infrastruktur klammert Persistenz des Aggregats **und** das Einreihen des
/// „Eingelesen"-Ereignisses in **eine** Transaktion,
/// [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)). Darum ist
/// `store` **fehlerfrei** (kein `Result`): das Staging kann nicht fachlich scheitern, der Commit
/// (io) schon.
///
/// **Der Speicher vergibt die Identität:** er gibt die beim Ablegen zugeteilte
/// [`IngestionId`] zurück — die stabile, laufübergreifend eindeutige ID, unter der die
/// Roh-Erfassung liegt (io: `INSERT … RETURNING id`,
/// [0017](../../../docs/issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)).
/// Der Handler reicht genau diese ID an den [`EventSink`](crate::EventSink) weiter, sodass die
/// „Eingelesen"-Nachricht als **Zeiger** auf die abgelegte Roh-Zeile publiziert wird (Tabelle =
/// Wahrheit, Message = Pointer). Vorbild: `IFileGroupRepository.Add(group)` der C#-Referenz.
pub trait Store {
    /// Reiht das Aggregat in die offene Arbeitseinheit ein (Staging) und gibt die dabei vergebene
    /// [`IngestionId`] zurück.
    fn store(&self, aggregate: &LineAggregate) -> IngestionId;
}
