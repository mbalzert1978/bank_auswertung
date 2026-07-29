use std::cell::Cell;
use std::rc::Rc;

use auswerten_adapter::OccurrenceRow;

use crate::auswerten::OccurrenceSink;

/// Der **Abnehmer** der fenster-übergreifenden Reichweite im realen Lauf: er zählt die gelieferten
/// Vorkommen und wirft sie fort.
///
/// **Warum zählen und nicht mehr.** Diese Reichweite *gibt ihr Ergebnis zurück, statt es abzulegen* —
/// die Aggregate sind die alleinige Wahrheit. Wer damit rechnet, ist der Wiederkehr-Analyzer
/// ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)); bis der steht, hat der Lauf
/// niemanden, dem er die Zeitachse reichen könnte. Sie zu **zählen** macht immerhin beobachtbar, dass
/// die Reichweite lief und wie viel sie sah — ohne einen einzigen Wert zu drucken. Ein Vorkommen trägt
/// Fingerabdruck, Tag und Betrag; nichts davon gehört in eine Ausgabe, und die reine Anzahl ist kein
/// Rohwert.
///
/// `Clone` teilt denselben Zähler (`Rc` im Innern), sodass der Lauf ihn nach dem Registrieren des
/// Consumers noch auslesen kann.
#[derive(Clone, Debug, Default)]
pub struct CountingOccurrenceSink {
    count: Rc<Cell<usize>>,
}

impl CountingOccurrenceSink {
    /// Die Zahl der insgesamt entgegengenommenen Vorkommen.
    #[must_use]
    pub fn count(&self) -> usize {
        self.count.get()
    }
}

impl OccurrenceSink for CountingOccurrenceSink {
    fn receive(&self, occurrences: Vec<OccurrenceRow>) {
        self.count.set(self.count.get() + occurrences.len());
    }
}
