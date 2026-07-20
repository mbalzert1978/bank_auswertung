use application_kernel::Handler;
use auswerten_domain::{
    EvaluateHistory, EvaluationError, HistoryEvaluationHandler as DomainHandler,
};

use crate::abstractions::OccurrenceSeam;
use crate::adapter::OccurrenceBridge;
use crate::models::{EvaluateHistoryCommand, OccurrenceRow};

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
/// die alleinige Wahrheit. Er braucht darum auch **keine** Eigene-Konten-Config: er ordnet keine
/// Gegenkonten ein, er sammelt Vorkommen ein (einzweckig wie der erste Handler).
///
/// **Die Response ist die Sicht selbst** ([`OccurrenceRow`]s), nicht `()`: das Ergebnis wird
/// zurückgegeben, nicht gespeichert. Genau daraus folgt die Idempotenz — dieselbe Query über
/// dieselben unveränderlichen Aggregate liefert dieselbe Antwort, ohne Dedupe-Logik.
///
/// **Transaktions-blind** und terminal: er gibt seinen **nackten Domänen-Fehler**
/// [`EvaluationError`] zurück; `io` spiegelt dessen **redigierte** `Display`. Kein `OutboxError`,
/// keine Outbox-Mechanik.
pub struct HistoryEvaluationHandler<'a> {
    source: OccurrenceBridge<'a>,
}

impl<'a> HistoryEvaluationHandler<'a> {
    /// Verdrahtet den Handler gegen den öffentlichen Lese-Seam.
    pub fn new(source: &'a dyn OccurrenceSeam) -> Self {
        Self {
            source: OccurrenceBridge { seam: source },
        }
    }
}

impl Handler for HistoryEvaluationHandler<'_> {
    type Command = EvaluateHistoryCommand;
    type Response = Vec<OccurrenceRow>;
    type Error = EvaluationError;

    fn handle(
        &self,
        command: EvaluateHistoryCommand,
    ) -> Result<Vec<OccurrenceRow>, EvaluationError> {
        Ok(DomainHandler::new(&self.source)
            .handle(EvaluateHistory::new(command.span()))?
            .iter()
            .map(OccurrenceRow::from)
            .collect())
    }
}
