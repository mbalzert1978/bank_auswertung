use crate::entities::WindowEvaluation;
use crate::error::EvaluationError;
use crate::models::EvaluateWindow;
use crate::ports::{AnalysisWindowSource, EvaluationRepository};

/// Orchestriert die **pro-Fenster-Reichweite** (ADR-0013, Entscheidung E, Reichweite 1) — er **lebt
/// in der Domäne** und hängt **nur** an den von ihr verlangten Ports; die Verdrahtung an die
/// Außenwelt (io/Fakes) besorgt der Adapter über Port-Bridges.
///
/// **Ablauf je Kommando** — die uniforme Handler-Form aller Slices:
/// 1. das eingefrorene Auswertungsfenster **per ID lesen** ([`AnalysisWindowSource`]) — herein kam
///    ein **Zeiger**, kein eingebettetes Aggregat (ADR-0012),
/// 2. daraus die periodische Kennzahl **innerhalb** des Fensters rechnen (Cashflow je Monat),
/// 3. das Ergebnis **ablegen** ([`EvaluationRepository`]).
///
/// **Kein Publish:** diese Reichweite stößt nichts stromabwärts an. Die zweite, fenster-übergreifende
/// Reichweite hängt nicht an ihr, sondern an `fenstern`s `HistorieVollständig` — genau deshalb, weil
/// nur `fenstern` die Erschöpfung der Historie kennt. Eine Nachricht ohne Empfänger zu publizieren
/// wäre Mechanik auf Vorrat.
///
/// Der Handler kennt **keine** Transaktion: die Ablage *staged* nur, den atomaren Commit besorgt io.
/// Lesen und Ablegen sind je fehlbar und terminal; ihr [`EvaluationError`] propagiert per `?` — kein
/// Dead-Letter, kein Retry (ADR-0012).
pub struct WindowEvaluationHandler<'a> {
    source: &'a dyn AnalysisWindowSource,
    repository: &'a dyn EvaluationRepository,
}

impl<'a> WindowEvaluationHandler<'a> {
    pub fn new(
        source: &'a dyn AnalysisWindowSource,
        repository: &'a dyn EvaluationRepository,
    ) -> Self {
        Self { source, repository }
    }

    /// Verarbeitet das Kommando: Fenster per ID lesen → Cashflow je Monat rechnen → (staged)
    /// ablegen. `Ok(())` = verarbeitet; `Err(EvaluationError)` = terminaler Fehler (der Adapter
    /// reicht ihn an io).
    pub fn handle(&self, command: EvaluateWindow) -> Result<(), EvaluationError> {
        let window = self.source.read(command.window())?;
        self.repository
            .save(&WindowEvaluation::of(command.into_window(), &window))
    }
}
