use application_kernel::Handler;
use fenstern_domain::{StartAnalysis, StartHandler as DomainHandler, WindowError};

use crate::abstractions::{ObfuscatedWindowSeam, WindowPublishSeam};
use crate::adapter::{AdvanceWindowBridge, AnchorBridge};
use crate::models::StartAnalysisCommand;

/// Der **zweite `application_kernel::Handler` des Slice**: die reine
/// `Command → Domäne → Response`-Pipeline des **Anstoßes** (ADR-0011, Entscheidung A; ADR-0013,
/// Entscheidung B/D). `io`s generische `OutboxConsumer` fährt ihn über den
/// [`StartAnalysisCommandDecoder`](crate::StartAnalysisCommandDecoder), registriert auf
/// [`START_MESSAGE_TYPE`](crate::START_MESSAGE_TYPE).
///
/// Er ist **einzweckig**: er eröffnet den Lauf und nichts sonst — darum hängt er nur an **zwei**
/// Bridges (Anker lesen, Fenster-Kommando publizieren), wo der
/// [`WindowingHandler`](crate::WindowingHandler) sechs braucht. Er kennt insbesondere weder Ablage
/// noch Seitengröße: welche Zeilen in ein Fenster fallen, entscheidet der Fenster-Schritt.
///
/// **Transaktions-blind** wie sein Geschwister: `publish` *staged* nur, die `UnitOfWork` besitzt
/// `io`. Er gibt seinen **nackten Domänen-Fehler** [`WindowError`] zurück; jeder Fehler ist
/// terminal. Ein **leerer Bestand** ist keiner — dann endet er mit `Ok(())`, ohne zu publizieren.
pub struct StartHandler<'a> {
    anchors: AnchorBridge<'a>,
    advance: AdvanceWindowBridge<'a>,
}

impl<'a> StartHandler<'a> {
    /// Verdrahtet den Anstoß gegen die öffentlichen Seams — denselben Lese- und Publish-Seam, die
    /// auch der Fenster-Schritt nutzt (eine Ablage, eine Outbox).
    pub fn new(source: &'a dyn ObfuscatedWindowSeam, publish: &'a dyn WindowPublishSeam) -> Self {
        Self {
            anchors: AnchorBridge { seam: source },
            advance: AdvanceWindowBridge { seam: publish },
        }
    }
}

impl Handler for StartHandler<'_> {
    type Command = StartAnalysisCommand;
    type Response = ();
    type Error = WindowError;

    fn handle(&self, command: StartAnalysisCommand) -> Result<(), WindowError> {
        DomainHandler::new(&self.anchors, &self.advance)
            .handle(StartAnalysis::new(command.run().clone()))
    }
}
