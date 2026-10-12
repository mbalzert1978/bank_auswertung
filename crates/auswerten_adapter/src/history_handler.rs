use application_kernel::Handler;
use auswerten_domain::{
    EvaluateHistory, EvaluationError, HistoryEvaluationHandler as DomainHandler,
};

use crate::abstractions::OccurrenceSeam;
use crate::adapter::OccurrenceBridge;
use crate::models::{
    EvaluateHistoryCommand, HistoryEvaluationRow, OwnAccountsConfig, SavingsThresholdConfig,
};

/// Der **`application_kernel::Handler` des Slice** für die **fenster-übergreifende** Reichweite —
/// das Geschwister des [`WindowEvaluationHandler`](crate::WindowEvaluationHandler) und wie dieses
/// eine reine `Command → Domäne → Response`-Pipeline (ADR-0011, Entscheidung A).
///
/// **Er hängt an `HistorieVollständig`**
/// ([`CONSUMED_HISTORY_MESSAGE_TYPE`](crate::CONSUMED_HISTORY_MESSAGE_TYPE)), nicht an
/// `FensterGeschlossen` — die beiden Dispatch-Keys sind verschieden, ein geschlossenes Fenster
/// erreicht ihn also nie.
///
/// **Er verdrahtet genau einen Seam**, den Lese-Seam: diese Reichweite legt **nichts** ab. Kein
/// Vorkommens-/Fold-Log, keine Akkumulation, keine zweite persistierte Sicht — die Aggregate sind
/// die alleinige Wahrheit. Dazu die **Eigene-Konten-Config als Wert**: die Wiederkehr-Erkennung
/// nettet Umbuchungen aufs eigene Konto heraus, und dafür ordnet sie Gegenkonten auf Token ein — mit
/// derselben Regel und derselben Seed-Marker-Prüfung wie die pro-Fenster-Reichweite.
///
/// **Die Response ist die Sicht selbst** ([`HistoryEvaluationRow`]: Vorkommen, Wiederkehr und der
/// Aderlass samt Pareto-Ranking und Spar-Kandidaten über der Spar-Schwelle), nicht
/// `()`: das Ergebnis wird zurückgegeben, nicht gespeichert. Genau daraus folgt die Idempotenz —
/// dieselbe Query über dieselben unveränderlichen Aggregate liefert dieselbe Antwort, ohne
/// Dedupe-Logik.
///
/// **Transaktions-blind** und terminal: er gibt seinen **nackten Domänen-Fehler**
/// [`EvaluationError`] zurück; `io` spiegelt dessen **redigierte** `Display`. Kein `OutboxError`,
/// keine Outbox-Mechanik.
pub struct HistoryEvaluationHandler<'a> {
    /// Der Lese-Seam — die Bridge darüber entsteht **je Kommando**, weil sie dessen Lauf als Scope
    /// trägt (ADR-0016).
    seam: &'a dyn OccurrenceSeam,
    /// Die Eigene-Konten-Config — als rohes DTO verdrahtet und je `handle` gedeutet (die Deutung ist
    /// fehlbar, ein Konstruktor wäre es nicht).
    own_accounts: OwnAccountsConfig,
    /// Die Spar-Schwelle — ab welchem Jahresbetrag ein Kostentreiber vorgeschlagen wird.
    savings_threshold: SavingsThresholdConfig,
}

impl<'a> HistoryEvaluationHandler<'a> {
    /// Verdrahtet den Handler gegen den öffentlichen Lese-Seam, die Eigene-Konten-Config und die
    /// Spar-Schwelle.
    pub fn new(
        source: &'a dyn OccurrenceSeam,
        own_accounts: OwnAccountsConfig,
        savings_threshold: SavingsThresholdConfig,
    ) -> Self {
        Self {
            seam: source,
            own_accounts,
            savings_threshold,
        }
    }
}

impl Handler for HistoryEvaluationHandler<'_> {
    type Command = EvaluateHistoryCommand;
    type Response = HistoryEvaluationRow;
    type Error = EvaluationError;

    fn handle(
        &self,
        command: EvaluateHistoryCommand,
    ) -> Result<HistoryEvaluationRow, EvaluationError> {
        // **Zuerst** die Config deuten: eine fehlerhafte Config ist terminal, **bevor** gelesen wird.
        let own_accounts = self.own_accounts.to_own_accounts()?;
        let source = OccurrenceBridge {
            seam: self.seam,
            run: command.run(),
        };
        DomainHandler::new(
            &source,
            &own_accounts,
            self.savings_threshold.to_threshold(),
        )
        .handle(EvaluateHistory::new(command.span()))
        .map(|evaluation| HistoryEvaluationRow::from(&evaluation))
    }
}
