use crate::ports::{Calendar, EventSink, Money, RawLineSource, Store};
use crate::{LineAggregate, ReadError, ReadTransactions};

/// Orchestriert das Einlesen — er **lebt in der Domäne** und hängt **nur** an den von ihr
/// verlangten Ports (`RawLineSource`, `Calendar`, `Money`, `Store`, `EventSink`); die
/// Verdrahtung an die Außenwelt (io/Fakes) besorgt der Adapter über Port-Bridges. Kein
/// geteiltes `Handler`/`Command`-Interface (das C#-Mediator-/DI-Residuum trägt in Rust
/// nichts): der Adapter hält den **konkreten** Handler und ruft `handle` direkt.
///
/// **Ablauf je Kommando:** erst den Batch über [`RawLineSource::fetch`] **beschaffen**
/// (das Kommando ist seit
/// [Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
/// reiner Auslöser und reicht nur die Konfiguration durch), dann **alle** Zeilen zu
/// Aggregaten deuten (`collect::<Result<Vec<_>,_>>()`), dann je Aggregat **staged**
/// speichern — der [`Store`](crate::Store) vergibt dabei die
/// [`IngestionId`](crate::IngestionId) — und **genau diese ID** als „Eingelesen"-Zeiger
/// veröffentlichen (Tabelle = Wahrheit, Message = Pointer). Weil erst gebaut und dann
/// gestaged wird, hinterlässt ein Fehler nichts Halbes.
///
/// **Fehler bubblen auf, transaktions-blind:** scheitert die Beschaffung
/// ([`SourceError`](crate::SourceError)) oder eine beschaffte Zeile am Aggregat
/// ([`LineError`](crate::LineError)), ist das **terminal**; der Handler steigt mit der
/// Fehlersumme [`ReadError`] aus (der Adapter mappt sie auf `OutboxError::Warning`, io
/// dead-lettert). Der Handler kennt **keine** Transaktion: `store`/`publish` stagen nur,
/// den Commit der Arbeitseinheit (Aggregat **und** Ereignis atomar) besorgt io.
pub struct ReadHandler<'a> {
    source: &'a dyn RawLineSource,
    calendar: &'a dyn Calendar,
    money: &'a dyn Money,
    store: &'a dyn Store,
    event_sink: &'a dyn EventSink,
}

impl<'a> ReadHandler<'a> {
    pub fn new(
        source: &'a dyn RawLineSource,
        calendar: &'a dyn Calendar,
        money: &'a dyn Money,
        store: &'a dyn Store,
        event_sink: &'a dyn EventSink,
    ) -> Self {
        Self {
            source,
            calendar,
            money,
            store,
            event_sink,
        }
    }

    /// Verarbeitet den Auslöser: beschaffen → deuten → (staged) speichern + veröffentlichen.
    /// `Ok(())` = verarbeitet, `Err` = terminaler [`ReadError`] (bubblet über den Adapter
    /// zur Outbox).
    pub fn handle(&self, command: ReadTransactions) -> Result<(), ReadError> {
        self.source
            .fetch(&command.configuration)
            .map_err(ReadError::Source)?
            .into_iter()
            .map(|line| LineAggregate::from_raw_line(line, self.calendar, self.money))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ReadError::Line)?
            .into_iter()
            .for_each(|aggregate| {
                // Der Speicher vergibt die ID beim Ablegen; genau sie reist als Zeiger weiter.
                let id = self.store.store(&aggregate);
                self.event_sink.publish(&id);
            });
        Ok(())
    }
}
