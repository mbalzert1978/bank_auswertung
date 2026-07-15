use application_kernel::{Handler, Publisher};
use obfuskierung_domain::{BookingId, ObfuscateBooking, ObfuscationHandler, Seed, SourceError};

use crate::abstractions::{IngestedSourceSeam, ObfuscatedStoreSeam, ObfuscatorSeam};
use crate::bridges::{ObfuscatorBridge, PublisherBridge, RepositoryBridge, SourceBridge};
use crate::commands::{ObfuscateCommand, ObfuscatedMessage};

/// Der **`application_kernel::Handler` des Slice**: die reine `Command → Domäne → Response`-Pipeline,
/// über die `io`s generische `OutboxConsumer` den Slice fährt (ADR-0011, Entscheidung A). Er bündelt
/// die Port-Bridges (Quelle/Obfuskator/Repository/Publisher), die die öffentlichen Seams auf die
/// internen Domänen-Ports mappen, und delegiert an den domänen-internen
/// [`ObfuscationHandler`](obfuskierung_domain::ObfuscationHandler). Vorbild: der `ReadingHandler` des
/// Einlese-Slice.
///
/// **Transaktions-blind:** `save`/`publish` *stagen* nur; die eine `UnitOfWork` (Ablage + Nachricht
/// atomar) besitzt `io`. Der Handler gibt seinen **nackten Domänen-Fehler** [`SourceError`] zurück —
/// jeder Fehler ist terminal; `io` spiegelt dessen **redigierte** `Display`. Kein `OutboxError`, kein
/// `shared_kernel`.
///
/// Der `ObfuscationHandler` borgt die gebündelten Bridges; er wird pro `handle` frisch gebaut (die
/// Bridges tragen nur `&dyn`-Seams, das ist billig) — so bleibt der Handler ohne selbst-referenzielles
/// Feld. Den injizierten Seed hält er als Domänen-[`Seed`].
pub struct ObfuscatingHandler<'a> {
    seed: Seed,
    source: SourceBridge<'a>,
    obfuscator: ObfuscatorBridge<'a>,
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
        repository: &'a dyn ObfuscatedStoreSeam,
        publisher: &'a dyn Publisher<ObfuscatedMessage>,
    ) -> Self {
        Self {
            seed: Seed::new(seed),
            source: SourceBridge { seam: source },
            obfuscator: ObfuscatorBridge { seam: obfuscator },
            repository: RepositoryBridge { seam: repository },
            publisher: PublisherBridge { publisher },
        }
    }
}

impl Handler for ObfuscatingHandler<'_> {
    type Command = ObfuscateCommand;
    type Response = ();
    type Error = SourceError;

    fn handle(&self, command: ObfuscateCommand) -> Result<(), SourceError> {
        ObfuscationHandler::new(
            self.seed,
            &self.source,
            &self.obfuscator,
            &self.repository,
            &self.publisher,
        )
        .handle(ObfuscateBooking::new(BookingId::new(command.id())))
    }
}
