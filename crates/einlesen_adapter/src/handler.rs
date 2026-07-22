use application_kernel::Handler;
use einlesen_domain::{ReadError, ReadHandler, ReadTransactions, RunConfiguration};

use crate::abstractions::{
    CalendarSeam, CurrencySeam, MoneySeam, PublishSeam, SourceSeam, StoreSeam,
};
use crate::adapter::{
    CalendarBridge, CurrencyBridge, IngestedPublisher, MoneyBridge, SourceBridge, StoreBridge,
};
use crate::models::ReadCommand;

/// Der **`application_kernel::Handler` des Slice**: die reine `Command → Domäne → Response`-Pipeline,
/// über die `io`s generische `OutboxConsumer` den Slice fährt (ADR-0011, Entscheidung A). Er bündelt
/// die Port-Bridges (Quelle/Kalender/Geld/Speicher/Publisher), die die öffentlichen Seams auf die
/// internen Domänen-Ports mappen, und delegiert an den domänen-internen
/// [`ReadHandler`](einlesen_domain::ReadHandler).
///
/// **Transaktions-blind:** den atomaren Commit der Arbeitseinheit (Aggregat + Nachricht) besitzt
/// `io`. Der Handler gibt seinen **nackten Domänen-Fehler** [`ReadError`] zurück — jeder Fehler
/// ist terminal; `io` spiegelt dessen **redigierte** `Display`. Kein `OutboxError`, keine
/// Outbox-Mechanik.
///
/// Der `ReadHandler` borgt die gebündelten Bridges; er wird pro `handle` frisch gebaut (die Bridges
/// tragen nur `&dyn`-Seams, das ist billig) — so bleibt der Handler ohne selbst-referenzielles Feld.
pub struct ReadingHandler<'a> {
    source: SourceBridge<'a>,
    calendar: CalendarBridge<'a>,
    money: MoneyBridge<'a>,
    currency: CurrencyBridge<'a>,
    store: StoreBridge<'a>,
    publisher: IngestedPublisher<'a>,
}

impl<'a> ReadingHandler<'a> {
    /// Verdrahtet den Handler gegen die öffentlichen Seams (die `io` oder ein Slice-Test erfüllt).
    /// Die Bridges mappen sie auf die internen Domänen-Ports; die `CurrencyBridge` löst die Währung
    /// samt Minor Unit auf, mit der die `MoneyBridge` den Betrag währungs-bewusst skaliert.
    pub fn new(
        source: &'a dyn SourceSeam,
        calendar: &'a dyn CalendarSeam,
        money: &'a dyn MoneySeam,
        currency: &'a dyn CurrencySeam,
        store: &'a dyn StoreSeam,
        publisher: &'a dyn PublishSeam,
    ) -> Self {
        Self {
            source: SourceBridge { seam: source },
            calendar: CalendarBridge { seam: calendar },
            money: MoneyBridge { seam: money },
            currency: CurrencyBridge { seam: currency },
            store: StoreBridge { seam: store },
            publisher: IngestedPublisher { seam: publisher },
        }
    }
}

impl Handler for ReadingHandler<'_> {
    type Command = ReadCommand;
    type Response = ();
    type Error = ReadError;

    fn handle(&self, _command: ReadCommand) -> Result<(), ReadError> {
        // `ReadCommand` ist ein **feldloser** Auslöser (Issue 0018): bewusst verworfen, die
        // Quelle/Konfiguration beschafft der Handler über den Quell-Seam. Trägt das Kommando je
        // ein Feld, muss es hier statt der leeren `RunConfiguration` einfließen.
        ReadHandler::new(
            &self.source,
            &self.calendar,
            &self.money,
            &self.currency,
            &self.store,
            &self.publisher,
        )
        .handle(ReadTransactions::new(RunConfiguration))
    }
}
