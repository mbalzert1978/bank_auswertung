use application_kernel::{Handler, Publisher};
use obfuskierung_domain::{
    BookingId, ObfuscateBooking, ObfuscationError, ObfuscationHandler, Seed,
};

use crate::abstractions::{
    IngestedSourceSeam, KnownPersonsSeam, ObfuscatedStoreSeam, ObfuscatorSeam,
};
use crate::adapter::{
    ObfuscatorBridge, PersonsBridge, PublisherBridge, RepositoryBridge, SourceBridge,
};
use crate::models::{ObfuscateCommand, ObfuscatedMessage};

/// Der **`application_kernel::Handler` des Slice**: die reine `Command → Domäne → Response`-Pipeline,
/// über die `io`s generische `OutboxConsumer` den Slice fährt (ADR-0011, Entscheidung A). Er bündelt
/// die Port-Bridges (Quelle/Obfuskator/Namensliste/Repository/Publisher), die die öffentlichen Seams auf die
/// internen Domänen-Ports mappen, und delegiert an den domänen-internen
/// [`ObfuscationHandler`](obfuskierung_domain::ObfuscationHandler). Vorbild: der `ReadingHandler` des
/// Einlese-Slice.
///
/// **Transaktions-blind:** `save`/`publish` *stagen* nur; die eine `UnitOfWork` (Ablage + Nachricht
/// atomar) besitzt `io`. Der Handler gibt seinen **nackten Domänen-Fehler** [`ObfuscationError`] zurück —
/// jeder Fehler ist terminal; `io` spiegelt dessen **redigierte** `Display`. Kein `OutboxError`, keine
/// Outbox-Mechanik.
///
/// Der `ObfuscationHandler` borgt die gebündelten Bridges; er wird pro `handle` frisch gebaut (die
/// Bridges tragen nur `&dyn`-Seams, das ist billig) — so bleibt der Handler ohne selbst-referenzielles
/// Feld. Den injizierten Seed hält er als Domänen-[`Seed`].
pub struct ObfuscatingHandler<'a> {
    seed: Seed,
    source: SourceBridge<'a>,
    obfuscator: ObfuscatorBridge<'a>,
    persons: PersonsBridge<'a>,
    repository: RepositoryBridge<'a>,
    publisher: PublisherBridge<'a>,
}

impl<'a> ObfuscatingHandler<'a> {
    /// Verdrahtet den Handler gegen die öffentlichen Seams und den `application_kernel::Publisher`
    /// (den `io` oder ein Slice-Test erfüllt). Die Bridges mappen sie auf die internen Domänen-Ports.
    /// Der Seed reist als roher `u64` herein (io liest ihn aus der Config) und wird zur Domänen-[`Seed`]
    /// gehüllt.
    pub fn new(
        seed: u64,
        source: &'a dyn IngestedSourceSeam,
        obfuscator: &'a dyn ObfuscatorSeam,
        persons: &'a dyn KnownPersonsSeam,
        repository: &'a dyn ObfuscatedStoreSeam,
        publisher: &'a dyn Publisher<ObfuscatedMessage>,
    ) -> Self {
        Self {
            seed: Seed::new(seed),
            source: SourceBridge { seam: source },
            obfuscator: ObfuscatorBridge { seam: obfuscator },
            persons: PersonsBridge { seam: persons },
            repository: RepositoryBridge { seam: repository },
            publisher: PublisherBridge { publisher },
        }
    }
}

impl Handler for ObfuscatingHandler<'_> {
    type Command = ObfuscateCommand;
    type Response = ();
    type Error = ObfuscationError;

    fn handle(&self, command: ObfuscateCommand) -> Result<(), ObfuscationError> {
        ObfuscationHandler::new(
            self.seed,
            &self.source,
            &self.obfuscator,
            &self.persons,
            &self.repository,
            &self.publisher,
        )
        .handle(ObfuscateBooking::new(BookingId::new(command.id())))
    }
}
