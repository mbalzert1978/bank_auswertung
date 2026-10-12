use std::cell::Cell;
use std::rc::Rc;

use auswerten_adapter::HistoryEvaluationRow;

use crate::auswerten::HistoryEvaluationSink;

/// Der **Abnehmer** der fenster-übergreifenden Reichweite im realen Lauf: er zählt die gelieferten
/// Vorkommen und Spar-Kandidaten und wirft sie fort.
///
/// **Warum zählen und nicht mehr.** Diese Reichweite *gibt ihr Ergebnis zurück, statt es abzulegen* —
/// die Aggregate sind die alleinige Wahrheit. Die Wiederkehr-Erkennung
/// ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)) rechnet schon **in** der
/// Reichweite; wer ihr Ergebnis liest, ist der Bericht (Issue 0012) — bis der steht, hat der Lauf
/// niemanden, dem er es reichen könnte. Sie zu **zählen** macht immerhin beobachtbar, dass
/// die Reichweite lief und wie viel sie sah — ohne einen einzigen Wert zu drucken. Ein Vorkommen trägt
/// Fingerabdruck, Tag und Betrag; nichts davon gehört in eine Ausgabe, und die reine Anzahl ist kein
/// Rohwert.
///
/// `Clone` teilt denselben Zähler (`Rc` im Innern), sodass der Lauf ihn nach dem Registrieren des
/// Consumers noch auslesen kann.
#[derive(Clone, Debug, Default)]
pub struct CountingHistorySink {
    count: Rc<Cell<usize>>,
    candidates: Rc<Cell<usize>>,
}

impl CountingHistorySink {
    /// Die Zahl der insgesamt entgegengenommenen Vorkommen.
    #[must_use]
    pub fn count(&self) -> usize {
        self.count.get()
    }

    /// Die Zahl der insgesamt entgegengenommenen Spar-Kandidaten (Issue 0007).
    #[must_use]
    pub fn candidates(&self) -> usize {
        self.candidates.get()
    }
}

impl HistoryEvaluationSink for CountingHistorySink {
    fn receive(&self, evaluation: HistoryEvaluationRow) {
        self.count
            .set(self.count.get() + evaluation.occurrences.len());
        self.candidates
            .set(self.candidates.get() + evaluation.savings_candidates.len());
    }
}
