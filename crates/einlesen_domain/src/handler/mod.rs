use crate::ports::{Calendar, EventSink, Store};
use crate::{LineAggregate, LineError, ReadTransactions};

/// Orchestriert das Einlesen — er **lebt in der Domäne** und hängt **nur** an den von ihr
/// verlangten Ports (`Calendar`, `Store`, `EventSink`); die Verdrahtung an die
/// Außenwelt (io/Fakes) besorgt der Adapter über Port-Bridges. Kein geteiltes
/// `Handler`/`Command`-Interface (das C#-Mediator-/DI-Residuum trägt in Rust nichts): der
/// Adapter hält den **konkreten** Handler und ruft `handle` direkt.
///
/// **Ablauf je Kommando:** erst **alle** Zeilen zu Aggregaten deuten
/// (`collect::<Result<Vec<_>,_>>()?`), dann je Aggregat **staged** speichern und das
/// „Eingelesen"-Ereignis veröffentlichen. Weil erst gebaut und dann gestaged wird,
/// hinterlässt ein Fehler nichts Halbes.
///
/// **Fehler bubblen auf, transaktions-blind:** scheitert eine — von io bereits gelesene,
/// strukturell valide — Zeile am Aggregat, ist das **terminal**; der Handler steigt mit dem
/// `LineError` aus (der Adapter mappt ihn auf `OutboxError::Warning`, io dead-lettert). Der
/// Handler kennt **keine** Transaktion: `store`/`publish` stagen nur, den Commit der
/// Arbeitseinheit (Aggregat **und** Ereignis atomar) besorgt io.
pub struct ReadHandler<'a> {
    calendar: &'a dyn Calendar,
    store: &'a dyn Store,
    event_sink: &'a dyn EventSink,
}

impl<'a> ReadHandler<'a> {
    pub fn new(
        calendar: &'a dyn Calendar,
        store: &'a dyn Store,
        event_sink: &'a dyn EventSink,
    ) -> Self {
        Self {
            calendar,
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
            .map(|line| LineAggregate::from_raw_line(line, self.calendar))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .for_each(|aggregate| {
                self.store.store(&aggregate);
                self.event_sink.publish(&aggregate);
            });
        Ok(())
    }
}
