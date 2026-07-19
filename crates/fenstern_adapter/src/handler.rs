use std::num::NonZeroU32;

use application_kernel::Handler;
use fenstern_domain::{AdvanceWindow, WindowDays, WindowError, WindowingHandler as DomainHandler};

use crate::abstractions::{AnalysisWindowStoreSeam, ObfuscatedWindowSeam, WindowPublishSeam};
use crate::adapter::{
    AdvanceWindowBridge, AnchorBridge, HistoryCompleteBridge, RepositoryBridge, SourceBridge,
    WindowClosedBridge,
};
use crate::models::AdvanceWindowCommand;

/// Der **`application_kernel::Handler` des Slice**: die reine `Command → Domäne → Response`-Pipeline,
/// über die `io`s generische `OutboxConsumer` den Slice fährt (ADR-0011, Entscheidung A). Er bündelt
/// die Port-Bridges (Quelle/Repository/Publisher), die die öffentlichen Seams auf die internen
/// Domänen-Ports mappen, und delegiert an den domänen-internen `WindowingHandler`.
///
/// **Transaktions-blind:** `save`/`publish` *stagen* nur; die eine `UnitOfWork` (Ablage + Nachrichten
/// atomar) besitzt `io`. Der Handler gibt seinen **nackten Domänen-Fehler** [`WindowError`] zurück —
/// jeder Fehler ist terminal; `io` spiegelt dessen **redigierte** `Display`. Kein `OutboxError`,
/// keine Outbox-Mechanik.
///
/// Der domänen-interne Handler borgt die gebündelten Bridges; er wird pro `handle` frisch gebaut
/// (die Bridges tragen nur `&dyn`-Seams, das ist billig) — so bleibt der Handler ohne
/// selbst-referenzielles Feld. Die injizierte Seitengröße hält er als Domänen-[`WindowDays`].
pub struct WindowingHandler<'a> {
    window_days: WindowDays,
    source: SourceBridge<'a>,
    repository: RepositoryBridge<'a>,
    anchors: AnchorBridge<'a>,
    closed: WindowClosedBridge<'a>,
    advance: AdvanceWindowBridge<'a>,
    history: HistoryCompleteBridge<'a>,
}

impl<'a> WindowingHandler<'a> {
    /// Verdrahtet den Handler gegen die öffentlichen Seams. Die Seitengröße reist als
    /// [`NonZeroU32`] herein (**io liest die Config**) und wird zur Domänen-[`WindowDays`] gehüllt —
    /// eine Seitengröße null ist damit schon am Eingang unkonstruierbar, nicht „geprüft".
    pub fn new(
        window_days: NonZeroU32,
        source: &'a dyn ObfuscatedWindowSeam,
        repository: &'a dyn AnalysisWindowStoreSeam,
        publish: &'a dyn WindowPublishSeam,
    ) -> Self {
        Self {
            window_days: WindowDays::from(window_days),
            source: SourceBridge { seam: source },
            repository: RepositoryBridge { seam: repository },
            anchors: AnchorBridge { seam: source },
            closed: WindowClosedBridge { seam: publish },
            advance: AdvanceWindowBridge { seam: publish },
            history: HistoryCompleteBridge { seam: publish },
        }
    }
}

impl Handler for WindowingHandler<'_> {
    type Command = AdvanceWindowCommand;
    type Response = ();
    type Error = WindowError;

    fn handle(&self, command: AdvanceWindowCommand) -> Result<(), WindowError> {
        DomainHandler::new(
            self.window_days,
            &self.source,
            &self.repository,
            &self.anchors,
            &self.closed,
            &self.advance,
            &self.history,
        )
        .handle(AdvanceWindow::new(command.anchor(), command.run_top()))
    }
}
