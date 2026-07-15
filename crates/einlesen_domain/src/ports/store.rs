use crate::{IngestionId, LineAggregate, SinkError};

/// Der von der Domäne **verlangte** Port, um ein validiertes `LineAggregate` zu
/// **persistieren** und ihm dabei seine Identität zuzuteilen. Er ist **fehlbar** und
/// **terminal**: ein misslungener Zugriff ist ein [`SinkError::StoreFailed`], den der Handler
/// per `?` propagiert.
///
/// **Der Speicher vergibt die Identität:** er gibt die beim Ablegen zugeteilte
/// [`IngestionId`] zurück — die je abgelegter Roh-Zeile eindeutige ID, unter der die
/// Roh-Erfassung liegt (io: `INSERT … RETURNING id`). Der Handler reicht genau diese ID an
/// den [`EventSink`](crate::EventSink) weiter, sodass die „Eingelesen"-Nachricht als
/// **Zeiger** auf die abgelegte Roh-Zeile reist (Tabelle = Wahrheit, Message = Pointer). Den
/// atomaren Commit der Arbeitseinheit (Aggregat **und** Ereignis) besorgt io.
pub trait Store {
    /// Legt das Aggregat ab und gibt die dabei vergebene [`IngestionId`] zurück; `Err` = der
    /// Zugriff scheiterte terminal.
    fn store(&self, aggregate: &LineAggregate) -> Result<IngestionId, SinkError>;
}
