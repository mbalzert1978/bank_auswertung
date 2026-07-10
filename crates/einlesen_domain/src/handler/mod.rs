use crate::ports::{Calendar, EventSink, Money, Store};
use crate::{LineAggregate, LineError, ReadTransactions};

/// Orchestriert das Einlesen — er **lebt in der Domäne** und hängt **nur** an den von ihr
/// verlangten Ports (`Calendar`, `Money`, `Store`, `EventSink`); die Verdrahtung an die
/// Außenwelt (io/Fakes) besorgt der Adapter über Port-Bridges. Kein geteiltes
/// `Handler`/`Command`-Interface (das C#-Mediator-/DI-Residuum trägt in Rust nichts): der
/// Adapter hält den **konkreten** Handler und ruft `handle` direkt.
///
/// **Ablauf je Kommando:** erst **alle** Zeilen zu Aggregaten deuten
/// (`collect::<Result<Vec<_>,_>>()?`), dann je Aggregat **staged** speichern — der
/// [`Store`](crate::Store) vergibt dabei die [`IngestionId`](crate::IngestionId) — und **genau
/// diese ID** als „Eingelesen"-Zeiger veröffentlichen (Tabelle = Wahrheit, Message = Pointer).
/// Weil erst gebaut und dann gestaged wird, hinterlässt ein Fehler nichts Halbes.
///
/// **Fehler bubblen auf, transaktions-blind:** scheitert eine — von io bereits gelesene,
/// strukturell valide — Zeile am Aggregat, ist das **terminal**; der Handler steigt mit dem
/// `LineError` aus (der Adapter mappt ihn auf `OutboxError::Warning`, io dead-lettert). Der
/// Handler kennt **keine** Transaktion: `store`/`publish` stagen nur, den Commit der
/// Arbeitseinheit (Aggregat **und** Ereignis atomar) besorgt io.
pub struct ReadHandler<'a> {
    calendar: &'a dyn Calendar,
    money: &'a dyn Money,
    store: &'a dyn Store,
    event_sink: &'a dyn EventSink,
}

impl<'a> ReadHandler<'a> {
    pub fn new(
        calendar: &'a dyn Calendar,
        money: &'a dyn Money,
        store: &'a dyn Store,
        event_sink: &'a dyn EventSink,
    ) -> Self {
        Self {
            calendar,
            money,
            store,
            event_sink,
        }
    }

    /// Verarbeitet den Batch: deuten → (staged) speichern + veröffentlichen. `Ok(())` =
    /// verarbeitet, `Err` = terminaler `LineError` (bubblet über den Adapter zur Outbox).
    pub fn handle(&self, command: ReadTransactions) -> Result<(), LineError> {
        command
            .lines
            .into_iter()
            .map(|line| LineAggregate::from_raw_line(line, self.calendar, self.money))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .for_each(|aggregate| {
                // Der Speicher vergibt die ID beim Ablegen; genau sie reist als Zeiger weiter.
                let id = self.store.store(&aggregate);
                self.event_sink.publish(&id);
            });
        Ok(())
    }
}
