use crate::entities::AnalysisWindow;
use crate::error::WindowError;
use crate::models::AdvanceWindow;
use crate::ports::{
    AdvanceWindowPublisher, AnalysisWindowRepository, HistoryCompletePublisher,
    ObfuscatedWindowSource, WindowAnchor, WindowClosedPublisher,
};
use crate::value_objects::{DayStamp, WindowDays, WindowSpan};

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
/// 6. die Kette **fortsetzen oder abschließen** — siehe [`continuation`](Self::continuation).
///
/// Schritt 6 ist die **Selbst-Fortsetzung** (ADR-0013, Entscheidung B, Präzisierung 2): **ein**
/// Mechanismus, `max(booking_day < from)` über den [`WindowAnchor`]-Port, trägt Fortsetzung **und**
/// Terminierung. Genau **einer** der beiden Zweige feuert je Schritt — nie beide (keine gegabelte
/// Kette), nie keiner.
///
/// Der Handler kennt **keine** Transaktion: Ablage und Publish *stagen* nur, den atomaren Commit
/// besorgt io. Lesen, Ablegen und Publizieren sind je fehlbar und terminal; ihr [`WindowError`]
/// propagiert per `?` — kein Dead-Letter, kein Retry (ADR-0012).
pub struct WindowingHandler<'a> {
    window_days: WindowDays,
    source: &'a dyn ObfuscatedWindowSource,
    repository: &'a dyn AnalysisWindowRepository,
    anchors: &'a dyn WindowAnchor,
    closed: &'a dyn WindowClosedPublisher,
    advance: &'a dyn AdvanceWindowPublisher,
    history: &'a dyn HistoryCompletePublisher,
}

impl<'a> WindowingHandler<'a> {
    pub fn new(
        window_days: WindowDays,
        source: &'a dyn ObfuscatedWindowSource,
        repository: &'a dyn AnalysisWindowRepository,
        anchors: &'a dyn WindowAnchor,
        closed: &'a dyn WindowClosedPublisher,
        advance: &'a dyn AdvanceWindowPublisher,
        history: &'a dyn HistoryCompletePublisher,
    ) -> Self {
        Self {
            window_days,
            source,
            repository,
            anchors,
            closed,
            advance,
            history,
        }
    }

    /// Verarbeitet das Kommando: Seite aufspannen → lesen → Fenster bauen → (staged) ablegen → neue
    /// ID publizieren → Kette fortsetzen oder abschließen. `Ok(())` = verarbeitet;
    /// `Err(WindowError)` = terminaler Fehler (der Adapter reicht ihn an io).
    pub fn handle(&self, command: AdvanceWindow) -> Result<(), WindowError> {
        let span = WindowSpan::page(command.anchor(), self.window_days);
        let bookings = self.source.read(&span)?;
        let window = AnalysisWindow::open(span, bookings)?;
        let id = self.repository.save(&window)?;
        self.closed.publish(&id)?;
        self.continuation(span, command.run_top())
    }

    /// Die **Selbst-Fortsetzung** — der eine Mechanismus aus ADR-0013, Entscheidung B,
    /// Präzisierung 2: der nächste Anker ist der **jüngste echte Buchungstag unter der
    /// Fensterunterkante**.
    ///
    /// - liefert die Abfrage **einen** Tag → das nächste [`AdvanceWindow`] mit diesem Anker (der
    ///   Lauf-Gipfel reist unverändert mit);
    /// - liefert sie **nichts** → der Abschluss mit der **Lauf-Spanne** `[from … run_top]`.
    ///
    /// Weil der Anker ein **echter** Buchungstag ist, sitzt jede Folgeseite auf Daten: Lücken —
    /// auch größer als eine Seitengröße — werden **übersprungen** statt in leere Fenster gegossen,
    /// die Seiten **überlappen nicht**, und die Kette **terminiert**, weil der Anker echt unter die
    /// Unterkante fällt und damit **streng monoton sinkt**.
    fn continuation(&self, span: WindowSpan, run_top: DayStamp) -> Result<(), WindowError> {
        match self.anchors.latest_before(span.from())? {
            Some(next) => self.advance.publish(&AdvanceWindow::new(next, run_top)),
            None => self
                .history
                .publish(&WindowSpan::spanning(span.from(), run_top)),
        }
    }
}
