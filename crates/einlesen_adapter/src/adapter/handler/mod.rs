use application_kernel::{Handler, Publisher};
use einlesen_domain::{Kalender, ZeilenAggregat, ZeilenError};

use crate::eingelesen::Eingelesen;
use crate::umsaetze_einlesen::UmsätzeEinlesen;
use crate::zeile::Zeile;

/// Orchestriert das Einlesen: hängt **nur** an internen Domänen-Ports (`Kalender`, verlangt von
/// `einlesen_domain`) und dem injizierten `Publisher<Eingelesen>` — keine adapter-erfundenen
/// Ports (Vorbild `DownloaderHandler`: „the handler depends only on internal ports"). Die
/// Rohzeilen kommen bereits **mit dem Kommando** ([ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md):
/// „der Command trägt, was der Handler für seinen Task braucht — der ganze Batch, nicht
/// zeilenweise").
///
/// **Fehler bubblen auf, der Slice dead-lettert nicht selbst:** scheitert eine — von IO bereits
/// gelesene, strukturell valide — Zeile am Aggregat, ist das **terminal** und muss reportet
/// werden; der Handler steigt mit dem `ZeilenError` aus (er bubblet über die Consume-Bridge zu
/// `OutboxError::Warning`, und die IO-Outbox dead-lettert die Nachricht). Weil erst **alle**
/// Aggregate gebaut werden (`collect`) und danach publiziert wird, hinterlässt ein terminaler
/// Fehler nichts Halbes.
pub(crate) struct EinleseHandler<'a> {
    pub(crate) kalender: &'a dyn Kalender,
    pub(crate) publisher: &'a dyn Publisher<Eingelesen>,
}

impl Handler for EinleseHandler<'_> {
    type Command = UmsätzeEinlesen;
    type Response = ();
    type Error = ZeilenError;

    fn handle(&self, command: UmsätzeEinlesen) -> Result<Self::Response, Self::Error> {
        command
            .zeilen
            .iter()
            .map(|rohsatz| ZeilenAggregat::aus_rohzeile(rohsatz, self.kalender))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .for_each(|aggregat| {
                self.publisher
                    .publish(Eingelesen::new(Zeile::aus_aggregat(aggregat)));
            });
        Ok(())
    }
}
