use crate::error::EvaluationError;
use crate::models::EvaluateHistory;
use crate::ports::OccurrenceSource;
use crate::value_objects::{HistoryEvaluation, OwnAccounts, SavingsThreshold};

/// Orchestriert die **fenster-übergreifende** Reichweite (ADR-0013, Entscheidung E, Reichweite 2) —
/// der **zweite einzweckige Handler** des Slice, Geschwister des
/// [`WindowEvaluationHandler`](crate::WindowEvaluationHandler).
///
/// **Ablauf je Kommando** — kurz, weil es genau ein I/O gibt:
/// 1. die Historie **aller** Fenster des Laufs lesen ([`OccurrenceSource`]), gescopt über die
///    **Lauf-Spanne** aus dem Kommando,
/// 2. prüfen, dass Historie und Eigene-Konten-Config aus **derselben** Tokenisierung stammen,
/// 3. daraus die [`HistoryEvaluation`] bilden: die Vorkommen der ganzen Zeitachse, die
///    **wiederkehrenden Abflüsse** darin ([Issue 0005](../../../docs/issues/0005-wiederkehr-erkennung.md))
///    und den **Aderlass** — Cashflow je Konto und Monat, Pareto-Ranking, Spar-Kandidaten
///    ([Issue 0007](../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)).
///
/// **Er hängt an `HistorieVollständig`, nicht an `FensterGeschlossen`** — und das ist keine
/// Geschmacksfrage: `auswerten` kann die letzte Seite **nicht selbst erkennen**, weil der Pager
/// neu→alt läuft. Nur `fenstern` weiß, wann die Historie erschöpft ist. Liefe diese Reichweite je
/// geschlossenem Fenster, sähe sie nie mehr als das eine Fenster, das gerade fertig wurde — und ein
/// **jährliches** Abo erschiene darin nur einmal, also nie als Wiederkehr.
///
/// **Er legt nichts ab.** Es gibt hier keinen Schreib-Port, kein Vorkommens-/Fold-Log und keine
/// Akkumulation — die Aggregate sind die alleinige Wahrheit, diese Sicht ist ein reines **Read**.
/// Genau daraus folgt die Idempotenz: eine Query über **unveränderliche** Snapshots antwortet auf
/// erneute Zustellung identisch, ohne dass es dafür Dedupe-Logik bräuchte.
///
/// **Die Eigene-Konten-Config braucht er für das Netting:** eine wiederkehrende Umbuchung aufs eigene
/// Konto ist kein Kosten-Treiber. Er bekommt sie als **Wert**, wie sein Geschwister — die Domäne liest
/// keine Datei. Ebenso die [`SavingsThreshold`]: ab welchem Jahresbetrag ein Treiber als
/// Spar-Kandidat vorgeschlagen wird, ist Konfiguration des Nutzers, kein Port.
///
/// Lesen ist fehlbar und terminal; sein [`EvaluationError`] propagiert per `?` — kein Dead-Letter,
/// kein Retry (ADR-0012).
pub struct HistoryEvaluationHandler<'a> {
    source: &'a dyn OccurrenceSource,
    own_accounts: &'a OwnAccounts,
    savings_threshold: SavingsThreshold,
}

impl<'a> HistoryEvaluationHandler<'a> {
    pub fn new(
        source: &'a dyn OccurrenceSource,
        own_accounts: &'a OwnAccounts,
        savings_threshold: SavingsThreshold,
    ) -> Self {
        Self {
            source,
            own_accounts,
            savings_threshold,
        }
    }

    /// Verarbeitet das Kommando: die Historie der Lauf-Spanne lesen → Seed-Marker prüfen → Vorkommen
    /// und Wiederkehr bilden. `Ok(auswertung)` = die Sicht über die ganze Zeitachse;
    /// `Err(EvaluationError)` = terminaler Fehler (der Adapter reicht ihn an io).
    pub fn handle(&self, command: EvaluateHistory) -> Result<HistoryEvaluation, EvaluationError> {
        // Eine **leere** Historie trägt keinen Marker — und braucht keinen: ohne Buchung gibt es kein
        // Token, das sich falsch einordnen ließe. Trägt sie einen, muss die Config für ihn
        // nachgewiesen sein, sonst liefe jede Umbuchung lautlos als „fremd" durch und erschiene als
        // wiederkehrender Abfluss.
        match self.source.history_in(command.span())?.recorded() {
            None => Ok(HistoryEvaluation::empty()),
            Some((marker, bookings)) => Ok(HistoryEvaluation::of(
                bookings,
                self.own_accounts.verified_for(marker)?,
                self.savings_threshold,
            )),
        }
    }
}
