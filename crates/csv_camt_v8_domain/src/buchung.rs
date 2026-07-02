use crate::betrag::BetragEntity;
use crate::buchungsart::BuchungsartEntity;
use crate::empfänger::EmpfängerEntity;
use crate::gegenkonto::GegenkontoEntity;
use crate::konto_id::KontoIdEntity;
use crate::obfuskierter_text::ObfuskierterTextEntity;
use crate::richtung::RichtungEntity;

/// Ein einzelner Kontoumsatz als Aggregat der Domäne, zusammengesetzt aus den
/// bereits obfuskierten/validierten Wert-Typen. Jedes Feld ist an seinem eigenen
/// Rand entstanden — [`BuchungAggregat::new`] kann daher nicht fehlschlagen. Die
/// Felder sind gekapselt; nach außen liest der Adapter sie über die Getter und
/// projiziert sie auf das `Buchung`-DTO.
///
/// `verwendungszweck` ist `Option`: ein fehlender Freitext wird am Feld
/// modelliert, nicht als leerer [`ObfuskierterTextEntity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuchungAggregat {
    konto: KontoIdEntity,
    betrag: BetragEntity,
    richtung: RichtungEntity,
    empfänger: EmpfängerEntity,
    gegenkonto: GegenkontoEntity,
    verwendungszweck: Option<ObfuskierterTextEntity>,
    buchungsart: BuchungsartEntity,
}

impl BuchungAggregat {
    pub fn new(
        konto: KontoIdEntity,
        betrag: BetragEntity,
        richtung: RichtungEntity,
        empfänger: EmpfängerEntity,
        gegenkonto: GegenkontoEntity,
        verwendungszweck: Option<ObfuskierterTextEntity>,
        buchungsart: BuchungsartEntity,
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

    pub fn konto(&self) -> &KontoIdEntity {
        &self.konto
    }

    pub fn betrag(&self) -> BetragEntity {
        self.betrag
    }

    pub fn richtung(&self) -> RichtungEntity {
        self.richtung
    }

    pub fn empfänger(&self) -> &EmpfängerEntity {
        &self.empfänger
    }

    pub fn gegenkonto(&self) -> GegenkontoEntity {
        self.gegenkonto
    }

    pub fn verwendungszweck(&self) -> Option<&ObfuskierterTextEntity> {
        self.verwendungszweck.as_ref()
    }

    pub fn buchungsart(&self) -> BuchungsartEntity {
        self.buchungsart
    }
}
