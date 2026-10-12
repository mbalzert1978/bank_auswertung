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
///
/// **Ein neuer Import ergänzt** ([ADR-0016](../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)):
/// trägt der Bestand den [`RowKey`](crate::RowKey) des Aggregats schon, legt der Speicher **nichts**
/// an und antwortet [`Recording::AlreadyKnown`] — der Handler publiziert dann auch nichts.
pub trait Store {
    /// Legt das Aggregat ab und gibt die dabei vergebene [`IngestionId`] zurück — oder meldet,
    /// dass seine Zeile schon im Bestand liegt; `Err` = der Zugriff scheiterte terminal.
    fn store(&self, aggregate: &LineAggregate) -> Result<Recording, SinkError>;
}

/// Was die Ablage einer Zeile ergeben hat — ein **Summentyp** statt `Option<IngestionId>`, damit der
/// Handler beide Ausgänge beim Namen nennt (ADR-0005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recording {
    /// Die Zeile war neu: abgelegt unter der vom Speicher vergebenen ID, die als Zeiger reist.
    New(IngestionId),
    /// Die Zeile lag schon im Bestand (gleicher Zeilen-Schlüssel): nichts angelegt, nichts zu
    /// publizieren.
    AlreadyKnown,
}
