use crate::entities::AnalysisWindow;
use crate::error::WindowError;
use crate::models::AdvanceWindow;
use crate::ports::{
    AnalysisWindowRepository, HistoryCompletePublisher, ObfuscatedWindowSource,
    WindowClosedPublisher,
};
use crate::value_objects::{WindowDays, WindowSpan};

/// Orchestriert **einen Fenster-Schritt** — er **lebt in der Domäne** und hängt **nur** an den von
/// ihr verlangten Ports; die Verdrahtung an die Außenwelt (io/Fakes) besorgt der Adapter über
/// Port-Bridges. Die injizierte Seitengröße [`WindowDays`] hält er als Wert: **io liest die
/// Config**, die Domäne liest keine Datei (ADR-0013, Entscheidung B).
///
/// **Ablauf je Kommando** — die uniforme Handler-Form aller Slices, nur mit der Kardinalität N→1:
/// 1. die Seite `[anchor − window_days + 1 … anchor]` aufspannen,
/// 2. ihre obfuskierten Buchungen **lesen** ([`ObfuscatedWindowSource`]),
/// 3. daraus **ein** [`AnalysisWindow`] **bauen** (die Invarianten entstehen an der Konstruktion),
/// 4. es **ablegen** ([`AnalysisWindowRepository`], das die **neue ID** vergibt),
/// 5. **diese neue ID** als Zeiger **publizieren** ([`WindowClosedPublisher`]),
/// 6. die Kette **abschließen** ([`HistoryCompletePublisher`] mit der Lauf-Spanne).
///
/// Schritt 6 ist heute **bedingungslos**, weil dieser Schnitt **noch nicht paginiert**: die Kette
/// endet nach einer Seite. Sobald die Selbst-Fortsetzung dazukommt, tritt an seine Stelle die
/// Verzweigung „nächster Anker vorhanden → `AdvanceWindow`, sonst → Abschluss" (ADR-0013,
/// Entscheidung B).
///
/// Der Handler kennt **keine** Transaktion: Ablage und Publish *stagen* nur, den atomaren Commit
/// besorgt io. Lesen, Ablegen und Publizieren sind je fehlbar und terminal; ihr [`WindowError`]
/// propagiert per `?` — kein Dead-Letter, kein Retry (ADR-0012).
pub struct WindowingHandler<'a> {
    window_days: WindowDays,
    source: &'a dyn ObfuscatedWindowSource,
    repository: &'a dyn AnalysisWindowRepository,
    closed: &'a dyn WindowClosedPublisher,
    history: &'a dyn HistoryCompletePublisher,
}

impl<'a> WindowingHandler<'a> {
    pub fn new(
        window_days: WindowDays,
        source: &'a dyn ObfuscatedWindowSource,
        repository: &'a dyn AnalysisWindowRepository,
        closed: &'a dyn WindowClosedPublisher,
        history: &'a dyn HistoryCompletePublisher,
    ) -> Self {
        Self {
            window_days,
            source,
            repository,
            closed,
            history,
        }
    }

    /// Verarbeitet das Kommando: Seite aufspannen → lesen → Fenster bauen → (staged) ablegen → neue
    /// ID publizieren. `Ok(())` = verarbeitet; `Err(WindowError)` = terminaler Fehler (der Adapter
    /// reicht ihn an io).
    pub fn handle(&self, command: AdvanceWindow) -> Result<(), WindowError> {
        let span = WindowSpan::page(command.anchor(), self.window_days);
        let bookings = self.source.read(&span)?;
        let window = AnalysisWindow::open(span, bookings)?;
        let id = self.repository.save(&window)?;
        self.closed.publish(&id)?;
        self.history.publish(&span)?;
        Ok(())
    }
}
