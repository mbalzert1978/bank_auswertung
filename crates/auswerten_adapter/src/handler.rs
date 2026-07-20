use application_kernel::Handler;
use auswerten_domain::{EvaluateWindow, EvaluationError, WindowEvaluationHandler as DomainHandler};

use crate::abstractions::{
    AnalysisWindowSeam, CategorizerSeam, CategoryRegistrySeam, EvaluationStoreSeam,
};
use crate::adapter::{CategorizerBridge, RegistryBridge, RepositoryBridge, SourceBridge};
use crate::models::{ConfigRejected, EvaluateWindowCommand, OwnAccountsConfig};

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
    registry: RegistryBridge<'a>,
    categorizer: CategorizerBridge<'a>,
    /// Die Eigene-Konten-Config — als **Wert** verdrahtet, nicht als Port: sie steht beim Bau fest
    /// und gilt für den ganzen Lauf. Sie liegt hier noch als rohes DTO; gedeutet wird sie je
    /// `handle` (die Deutung ist **fehlbar**, ein Konstruktor wäre es nicht).
    own_accounts: OwnAccountsConfig,
}

impl<'a> WindowEvaluationHandler<'a> {
    /// Verdrahtet den Handler gegen die öffentlichen Seams.
    pub fn new(
        source: &'a dyn AnalysisWindowSeam,
        repository: &'a dyn EvaluationStoreSeam,
        registry: &'a dyn CategoryRegistrySeam,
        categorizer: &'a dyn CategorizerSeam,
        own_accounts: OwnAccountsConfig,
    ) -> Self {
        Self {
            source: SourceBridge { seam: source },
            repository: RepositoryBridge { seam: repository },
            registry: RegistryBridge { seam: registry },
            categorizer: CategorizerBridge { seam: categorizer },
            own_accounts,
        }
    }
}

impl Handler for WindowEvaluationHandler<'_> {
    type Command = EvaluateWindowCommand;
    type Response = ();
    type Error = EvaluationError;

    fn handle(&self, command: EvaluateWindowCommand) -> Result<(), EvaluationError> {
        // **Zuerst** die Config deuten: eine fehlerhafte Config ist terminal, **bevor** irgendetwas
        // gelesen oder abgelegt wird — sie beträfe jede Buchung des Fensters, nicht eine.
        let own_accounts =
            self.own_accounts
                .to_own_accounts()
                .map_err(|rejected| match rejected {
                    ConfigRejected::MissingMandatoryField { field } => {
                        EvaluationError::MissingMandatoryField { field }
                    }
                    ConfigRejected::UnknownAccountRole => EvaluationError::UnknownAccountRole,
                })?;
        DomainHandler::new(
            &self.source,
            &self.repository,
            &self.registry,
            &self.categorizer,
            &own_accounts,
        )
        .handle(EvaluateWindow::new(command.window()))
    }
}
