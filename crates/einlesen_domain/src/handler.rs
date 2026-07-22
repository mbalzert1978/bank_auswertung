use crate::ports::{Calendar, Currencies, EventSink, Money, RawLineSource, Store};
use crate::{LineAggregate, ReadError, ReadTransactions};

/// Orchestriert das Einlesen — er **lebt in der Domäne** und hängt **nur** an den von ihr
/// verlangten Ports (`RawLineSource`, `Calendar`, `Money`, `Currencies`, `Store`, `EventSink`); die
/// Verdrahtung an die Außenwelt (io/Fakes) besorgt der Adapter über Port-Bridges.
///
/// **Ablauf je Kommando:** erst den Batch über [`RawLineSource::fetch`] **beschaffen** (das
/// Kommando ist reiner Auslöser und reicht nur die Konfiguration durch), dann **alle** Zeilen
/// zu Aggregaten deuten (`collect::<Result<Vec<_>, _>>()`), dann je Aggregat **ablegen** — der
/// [`Store`](crate::Store) vergibt dabei die [`IngestionId`](crate::IngestionId) — und **genau
/// diese ID** als „Eingelesen"-Zeiger **veröffentlichen** (Tabelle = Wahrheit, Message =
/// Pointer). Weil erst **alle** Aggregate gebaut und dann abgelegt werden, hinterlässt eine
/// kaputte Zeile nichts Halbes: der Lauf steigt aus, bevor die erste Ablage passiert.
///
/// **Alle drei I/O-Schritte sind fehlbar und terminal:** scheitert Beschaffung
/// ([`SourceError`](crate::SourceError)), Deutung ([`LineError`](crate::LineError)) oder
/// Ablage/Veröffentlichung ([`SinkError`](crate::SinkError)), steigt der Handler mit der
/// Fehlersumme [`ReadError`] per `?` aus. Den atomaren Commit der Arbeitseinheit (Aggregat
/// **und** Ereignis) besorgt io.
pub struct ReadHandler<'a> {
    source: &'a dyn RawLineSource,
    calendar: &'a dyn Calendar,
    money: &'a dyn Money,
    currencies: &'a dyn Currencies,
    store: &'a dyn Store,
    event_sink: &'a dyn EventSink,
}

impl<'a> ReadHandler<'a> {
    pub fn new(
        source: &'a dyn RawLineSource,
        calendar: &'a dyn Calendar,
        money: &'a dyn Money,
        currencies: &'a dyn Currencies,
        store: &'a dyn Store,
        event_sink: &'a dyn EventSink,
    ) -> Self {
        Self {
            source,
            calendar,
            money,
            currencies,
            store,
            event_sink,
        }
    }

    /// Verarbeitet den Auslöser: beschaffen → deuten → ablegen → veröffentlichen.
    /// `Ok(())` = verarbeitet, `Err` = terminaler [`ReadError`].
    pub fn handle(&self, command: ReadTransactions) -> Result<(), ReadError> {
        self.source
            .fetch(&command.configuration)?
            .into_iter()
            .map(|line| {
                LineAggregate::from_raw_line(line, self.calendar, self.money, self.currencies)
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .try_for_each(|aggregate| -> Result<(), ReadError> {
                // Der Speicher vergibt die ID beim Ablegen; genau sie reist als Zeiger weiter.
                let id = self.store.store(&aggregate)?;
                self.event_sink.publish(&id)?;
                Ok(())
            })
    }
}
