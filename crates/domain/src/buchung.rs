use crate::{
    Betrag, Buchungsart, Empfänger, Fingerabdruck, Gegenkonto, KategorieAnfrage, KontoId,
    ObfuskierterText, Richtung,
};

/// Ein einzelner Kontoumsatz, zusammengesetzt aus den obfuskierten Value
/// Objects der Domäne. Jedes Feld ist bereits an seinem eigenen Rand
/// validiert — `Buchung::new` kann daher nicht fehlschlagen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buchung {
    konto: KontoId,
    betrag: Betrag,
    richtung: Richtung,
    empfänger: Empfänger,
    gegenkonto: Gegenkonto,
    verwendungszweck: ObfuskierterText,
    buchungsart: Buchungsart,
}

impl Buchung {
    pub fn new(
        konto: KontoId,
        betrag: Betrag,
        richtung: Richtung,
        empfänger: Empfänger,
        gegenkonto: Gegenkonto,
        verwendungszweck: ObfuskierterText,
        buchungsart: Buchungsart,
    ) -> Self {
        Self {
            konto,
            betrag,
            richtung,
            empfänger,
            gegenkonto,
            verwendungszweck,
            buchungsart,
        }
    }

    pub fn richtung(&self) -> Richtung {
        self.richtung
    }

    pub fn empfänger(&self) -> &Empfänger {
        &self.empfänger
    }

    pub fn verwendungszweck(&self) -> &ObfuskierterText {
        &self.verwendungszweck
    }

    /// Die Projektion `Buchung` → `KategorieAnfrage`: die schmale, eigene DTO
    /// des `Kategorisierer`-Ports.
    pub fn to_kategorie_anfrage(&self) -> KategorieAnfrage {
        KategorieAnfrage {
            fingerabdruck: Fingerabdruck::from_buchung(self),
            empfänger: ObfuskierterText::from(self.empfänger.identity_key().clone()),
            verwendungszweck: self.verwendungszweck.clone(),
        }
    }
}
