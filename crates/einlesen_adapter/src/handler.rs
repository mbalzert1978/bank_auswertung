use application_kernel::{Handler, Publisher};
use einlesen_domain::{ReadError, ReadHandler, ReadTransactions, RunConfiguration};

use crate::abstractions::{CalendarSeam, MoneySeam, SourceSeam, StoreSeam};
use crate::bridges::{CalendarBridge, IngestedPublisher, MoneyBridge, SourceBridge, StoreBridge};
use crate::commands::{IngestedMessage, ReadCommand};

/// Der **`application_kernel::Handler` des Slice**: die reine `Command → Domäne → Response`-Pipeline,
/// über die `io`s generische `OutboxConsumer` den Slice fährt (ADR-0011, Entscheidung A). Er bündelt
/// die Port-Bridges (Quelle/Kalender/Geld/Speicher/Publisher), die die öffentlichen Seams auf die
/// internen Domänen-Ports mappen, und delegiert an den domänen-internen
/// [`ReadHandler`](einlesen_domain::ReadHandler).
///
/// **Transaktions-blind:** `store`/`publish` *stagen* nur; die eine `UnitOfWork` (Consume-Ack +
/// Aggregat + Nachricht atomar) besitzt `io`. Der Handler gibt seinen **nackten Domänen-Fehler**
/// [`ReadError`] zurück — jeder Fehler ist terminal; `io` spiegelt dessen **redigierte** `Display`
/// als Dead-Letter-Grund (ADR-0011, Entscheidung B). Kein `OutboxError`, kein `shared_kernel`.
///
/// Der `ReadHandler` borgt die gebündelten Bridges; er wird pro `handle` frisch gebaut (die Bridges
/// tragen nur `&dyn`-Seams, das ist billig) — so bleibt der Handler ohne selbst-referenzielles Feld.
pub struct ReadingHandler<'a> {
    source: SourceBridge<'a>,
    calendar: CalendarBridge<'a>,
    money: MoneyBridge<'a>,
    store: StoreBridge<'a>,
    publisher: IngestedPublisher<'a>,
}

impl<'a> ReadingHandler<'a> {
    /// Verdrahtet den Handler gegen die öffentlichen Seams und den `application_kernel::Publisher`
    /// (den `io` oder ein Slice-Test erfüllt). Die Bridges mappen sie auf die internen Domänen-Ports.
    pub fn new(
        source: &'a dyn SourceSeam,
        calendar: &'a dyn CalendarSeam,
        money: &'a dyn MoneySeam,
        store: &'a dyn StoreSeam,
        publisher: &'a dyn Publisher<IngestedMessage>,
    ) -> Self {
        Self {
            source: SourceBridge { seam: source },
            calendar: CalendarBridge { seam: calendar },
            money: MoneyBridge { seam: money },
            store: StoreBridge { seam: store },
            publisher: IngestedPublisher { publisher },
        }
    }
}

impl Handler for ReadingHandler<'_> {
    type Command = ReadCommand;
    type Response = ();
    type Error = ReadError;

    fn handle(&self, _command: ReadCommand) -> Result<(), ReadError> {
        ReadHandler::new(
            &self.source,
            &self.calendar,
            &self.money,
            &self.store,
            &self.publisher,
        )
        .handle(ReadTransactions::new(RunConfiguration))
    }
}
