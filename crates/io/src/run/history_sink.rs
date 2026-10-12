use std::cell::RefCell;
use std::rc::Rc;

use auswerten_adapter::HistoryEvaluationRow;

use crate::auswerten::HistoryEvaluationSink;

/// Der **Abnehmer** der fenster-übergreifenden Reichweite im realen Lauf: er hält das zuletzt
/// gelieferte Ergebnis fest, bis der Lauf es abholt.
///
/// **Warum festhalten.** Diese Reichweite *gibt ihr Ergebnis zurück, statt es abzulegen* — die
/// Aggregate sind die alleinige Wahrheit. Ihr Leser ist der Bericht
/// ([Issue 0012](../../../../docs/issues/0012-html-bericht.md)): er zeigt genau dieses Ergebnis, statt
/// etwas nachzurechnen.
///
/// **Welcher Lauf.** Der Abnehmer weiß nicht, zu welchem Lauf ein Ergebnis gehört; das ergibt sich aus
/// den zwei Phasen von [`run`](super::run). Phase 1 arbeitet **alle** offenen Nachrichten ab — auch den
/// Abschluss eines nach einem Absturz liegengebliebenen alten Laufs —, und der Lauf verwirft, was bis
/// dahin ankam ([`take`](Self::take)). In Phase 2 liegt danach allein der Anstoß **dieses** Laufs und
/// seine Folge-Nachrichten in der Outbox; was dann ankommt, ist sein Ergebnis.
///
/// `Clone` teilt denselben Platz (`Rc` im Innern), sodass der Lauf ihn nach dem Registrieren des
/// Consumers noch auslesen kann.
#[derive(Clone, Debug, Default)]
pub struct RecordingHistorySink {
    latest: Rc<RefCell<Option<HistoryEvaluationRow>>>,
}

impl RecordingHistorySink {
    /// Holt das zuletzt entgegengenommene Ergebnis ab und leert den Platz — `None`, wenn seit dem
    /// letzten Abholen keines kam (etwa weil der Bestand leer war und kein Fenster entstand).
    #[must_use]
    pub fn take(&self) -> Option<HistoryEvaluationRow> {
        self.latest.borrow_mut().take()
    }
}

impl HistoryEvaluationSink for RecordingHistorySink {
    fn receive(&self, evaluation: HistoryEvaluationRow) {
        *self.latest.borrow_mut() = Some(evaluation);
    }
}
