use crate::MessageId;
use crate::outbox::{OutboxError, OutboxHandler, OutboxMessage, UnitOfWork};
use crate::sqlite::SqliteOutbox;

/// Der **Abräumer** für einen Nachrichtentyp, den heute niemand stromabwärts konsumiert: er quittiert
/// die Nachricht und tut sonst nichts.
///
/// **Wofür.** `obfuskieren` publiziert je Buchung ein `Obfuscated` — den Zeiger auf die Zeile hinter
/// der Datenschutz-Grenze. Der nächste Schritt `fenstern` hängt aber **nicht** daran: er paginiert
/// **einmal je Lauf** über die *Tabelle*, nicht je Buchung über die *Nachricht*
/// ([ADR-0013](../../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)). Ohne
/// einen Consumer bliebe darum jede dieser Nachrichten für immer offen, der Dispatcher meldete sie als
/// unzustellbar, und jeder spätere Lauf stellte den ganzen Altbestand erneut zu.
///
/// **Warum er kein Slice ist.** Er trifft keine fachliche Entscheidung und legt nichts ab — er
/// schließt allein den Buchungskreis der Outbox. Das ist Broker-Mechanik, und sie gehört damit hierher
/// und nicht in einen Slice.
///
/// **Er ist keine Dauerlösung, sondern eine ehrliche.** Sobald ein Schritt entsteht, der `Obfuscated`
/// wirklich verarbeitet, tritt er an diese Stelle — die Verdrahtung ändert sich dann um eine Zeile.
pub struct AckSink {
    outbox: SqliteOutbox,
}

impl AckSink {
    /// Bindet den Abräumer an das Rückgrat, auf dem er quittiert.
    #[must_use]
    pub fn new(outbox: SqliteOutbox) -> Self {
        Self { outbox }
    }
}

impl OutboxHandler for AckSink {
    /// Öffnet die Arbeitseinheit **allein für den Ack** und committet sie. Ein gescheiterter Commit ist
    /// — wie überall — ein io-interner [`OutboxError::Retry`]: die Nachricht bleibt offen und wird
    /// erneut zugestellt, bis der Broker sie nach der Höchstzahl an Versuchen aussondert.
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        self.outbox
            .begin(MessageId(&message.id))
            .commit()
            .map_err(|error| OutboxError::Retry {
                reason: error.to_string(),
            })
    }
}
