use application_kernel::Handler;
use auswerten_domain::{EvaluateWindow, EvaluationError, WindowEvaluationHandler as DomainHandler};

use crate::abstractions::{AnalysisWindowSeam, EvaluationStoreSeam};
use crate::adapter::{RepositoryBridge, SourceBridge};
use crate::models::EvaluateWindowCommand;

/// Der **`application_kernel::Handler` des Slice** für die **pro-Fenster**-Reichweite: die reine
/// `Command → Domäne → Response`-Pipeline, über die `io`s generische `OutboxConsumer` den Slice
/// fährt (ADR-0011, Entscheidung A). Er bündelt die Port-Bridges (Quelle/Repository), die die
/// öffentlichen Seams auf die internen Domänen-Ports mappen, und delegiert an den domänen-internen
/// `WindowEvaluationHandler`.
///
/// **Transaktions-blind:** `save` *staged* nur; die eine `UnitOfWork` besitzt `io`. Der Handler gibt
/// seinen **nackten Domänen-Fehler** [`EvaluationError`] zurück — jeder Fehler ist terminal; `io`
/// spiegelt dessen **redigierte** `Display`. Kein `OutboxError`, keine Outbox-Mechanik.
///
/// Der domänen-interne Handler borgt die gebündelten Bridges; er wird pro `handle` frisch gebaut
/// (die Bridges tragen nur `&dyn`-Seams, das ist billig) — so bleibt der Handler ohne
/// selbst-referenzielles Feld.
pub struct WindowEvaluationHandler<'a> {
    source: SourceBridge<'a>,
    repository: RepositoryBridge<'a>,
}

impl<'a> WindowEvaluationHandler<'a> {
    /// Verdrahtet den Handler gegen die öffentlichen Seams.
    pub fn new(
        source: &'a dyn AnalysisWindowSeam,
        repository: &'a dyn EvaluationStoreSeam,
    ) -> Self {
        Self {
            source: SourceBridge { seam: source },
            repository: RepositoryBridge { seam: repository },
        }
    }
}

impl Handler for WindowEvaluationHandler<'_> {
    type Command = EvaluateWindowCommand;
    type Response = ();
    type Error = EvaluationError;

    fn handle(&self, command: EvaluateWindowCommand) -> Result<(), EvaluationError> {
        DomainHandler::new(&self.source, &self.repository)
            .handle(EvaluateWindow::new(command.window()))
    }
}
