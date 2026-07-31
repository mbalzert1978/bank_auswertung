use crate::error::EvaluationError;
use crate::models::EvaluateHistory;
use crate::ports::OccurrenceSource;
use crate::value_objects::{Fingerprint, Occurrence, WindowedBooking};

/// Orchestriert die **fenster-übergreifende** Reichweite (ADR-0013, Entscheidung E, Reichweite 2) —
/// der **zweite einzweckige Handler** des Slice, Geschwister des
/// [`WindowEvaluationHandler`](crate::WindowEvaluationHandler).
///
/// **Ablauf je Kommando** — kurz, weil es genau ein I/O gibt:
/// 1. die Buchungen **aller** Fenster des Laufs lesen ([`OccurrenceSource`]), gescopt über die
///    **Lauf-Spanne** aus dem Kommando,
/// 2. jede zu ihrem [`Occurrence`] verdichten — der [`Fingerprint`] entsteht dabei **hier**, über
///    dieselbe reine Domänen-Funktion, die auch die pro-Fenster-Reichweite nutzt.
///
/// **Er hängt an `HistorieVollständig`, nicht an `FensterGeschlossen`** — und das ist keine
/// Geschmacksfrage: `auswerten` kann die letzte Seite **nicht selbst erkennen**, weil der Pager
/// neu→alt läuft. Nur `fenstern` weiß, wann die Historie erschöpft ist. Liefe diese Reichweite je
/// geschlossenem Fenster, sähe sie nie mehr als das eine Fenster, das gerade fertig wurde.
///
/// **Er legt nichts ab.** Es gibt hier keinen Schreib-Port, kein Vorkommens-/Fold-Log und keine
/// Akkumulation — die Aggregate sind die alleinige Wahrheit, diese Sicht ist ein reines **Read**.
/// Genau daraus folgt die Idempotenz: eine Query über **unveränderliche** Snapshots antwortet auf
/// erneute Zustellung identisch, ohne dass es dafür Dedupe-Logik bräuchte.
///
/// **Noch keine Wiederkehr:** dieses Ticket liefert die *Reichweite*, nicht den Rhythmus. Die
/// Wiederkehr-Erkennung ([0005](../../../docs/issues/0005-wiederkehr-erkennung.md)) setzt auf den
/// zurückgegebenen Vorkommen auf.
///
/// Lesen ist fehlbar und terminal; sein [`EvaluationError`] propagiert per `?` — kein Dead-Letter,
/// kein Retry (ADR-0012).
pub struct HistoryEvaluationHandler<'a> {
    source: &'a dyn OccurrenceSource,
}

impl<'a> HistoryEvaluationHandler<'a> {
    pub fn new(source: &'a dyn OccurrenceSource) -> Self {
        Self { source }
    }

    /// Verarbeitet das Kommando: alle Buchungen der Lauf-Spanne lesen → zu Vorkommen verdichten.
    /// `Ok(vorkommen)` = die Sicht über die ganze Zeitachse; `Err(EvaluationError)` = terminaler
    /// Fehler (der Adapter reicht ihn an io).
    pub fn handle(&self, command: EvaluateHistory) -> Result<Vec<Occurrence>, EvaluationError> {
        let mut occurrences = self
            .source
            .bookings_in(command.span())?
            .iter()
            .filter_map(Self::occurrence_of)
            .collect::<Vec<_>>();
        // **Chronologisch** — die Zeitachse ist der Sinn dieser Sicht, und erst auf ihr wird ein
        // Rhythmus sichtbar. Die Ordnung ist zudem **total** (Tag, dann Schlüssel, dann Zahlen), die
        // Sortierung also eingabe-unabhängig: dieselben Aggregate ergeben dieselbe Reihenfolge,
        // egal in welcher Reihenfolge io die Zeilen liefert.
        occurrences.sort();
        Ok(occurrences)
    }

    /// Verdichtet eine gelesene Buchung zu ihrem Vorkommen. Eine Buchung **ohne** Gegenpartei
    /// (Entgeltzeile) trägt **keines** bei: es gibt nichts, was ihre Art unterscheidbar machte, und
    /// ein Platzhalter-Schlüssel verklumpte alle Entgeltzeilen zu einer scheinbar wiederkehrenden
    /// Art — genau die Sorte stiller Falschaussage, die dieser Slice vermeidet.
    fn occurrence_of(booking: &WindowedBooking) -> Option<Occurrence> {
        Fingerprint::of(booking.counterparty()).map(|fingerprint| {
            Occurrence::new(
                booking.day(),
                fingerprint,
                booking.direction(),
                booking.amount(),
            )
        })
    }
}
