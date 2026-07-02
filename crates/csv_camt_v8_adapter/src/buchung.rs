use csv_camt_v8_domain::BuchungAggregat;

use crate::betrag::Betrag;
use crate::buchungsart::Buchungsart;
use crate::empfänger::Empfänger;
use crate::gegenkonto::Gegenkonto;
use crate::konto_id::KontoId;
use crate::obfuskierter_text::ObfuskierterText;
use crate::richtung::Richtung;

/// Das UBQL-DTO für einen obfuskierten Kontoumsatz — der **Ausgang** des Slice
/// ([ADR-0002](../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)):
/// die einzige Form, in der eine Buchung die Slice-Grenze verlässt. Trägt
/// ausschließlich obfuskierte/öffentliche Werte (`KONTO_x`, `PERSON_x`,
/// Gläubiger-ID) — nie eine rohe IBAN/BIC. Round-trip-vollständig geschnitten, so
/// dass sich das [`BuchungAggregat`] bei späterer Persistenz verlustfrei
/// rehydrieren lässt (Rehydrate-Invariante, ADR-0007).
///
/// `verwendungszweck` ist `Option`: ein fehlender Freitext wird am Feld
/// modelliert, nicht als leerer [`ObfuskierterText`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buchung {
    konto: KontoId,
    betrag: Betrag,
    richtung: Richtung,
    empfänger: Empfänger,
    gegenkonto: Gegenkonto,
    verwendungszweck: Option<ObfuskierterText>,
    buchungsart: Buchungsart,
}

impl Buchung {
    /// Baut das DTO aus seinen (bereits obfuskierten) Teil-DTOs — etwa zum Aufbau
    /// erwarteter Werte in Tests, oder zur Rehydration aus einem Store.
    pub fn new(
        konto: KontoId,
        betrag: Betrag,
        richtung: Richtung,
        empfänger: Empfänger,
        gegenkonto: Gegenkonto,
        verwendungszweck: Option<ObfuskierterText>,
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

    /// Projiziert das Domänen-Aggregat auf das Ausgangs-DTO — crate-intern, nach
    /// dem obfuskierenden Mappen.
    pub(crate) fn from_aggregat(aggregat: &BuchungAggregat) -> Self {
        Self {
            konto: KontoId::from_entity(aggregat.konto().clone()),
            betrag: Betrag::from_entity(aggregat.betrag()),
            richtung: Richtung::from_entity(aggregat.richtung()),
            empfänger: Empfänger::from_entity(aggregat.empfänger()),
            gegenkonto: Gegenkonto::from_entity(aggregat.gegenkonto()),
            verwendungszweck: aggregat
                .verwendungszweck()
                .cloned()
                .map(ObfuskierterText::from_entity),
            buchungsart: Buchungsart::from_entity(aggregat.buchungsart()),
        }
    }

    pub fn konto(&self) -> &KontoId {
        &self.konto
    }

    pub fn betrag(&self) -> Betrag {
        self.betrag
    }

    pub fn richtung(&self) -> Richtung {
        self.richtung
    }

    pub fn empfänger(&self) -> &Empfänger {
        &self.empfänger
    }

    pub fn gegenkonto(&self) -> Gegenkonto {
        self.gegenkonto
    }

    pub fn verwendungszweck(&self) -> Option<&ObfuskierterText> {
        self.verwendungszweck.as_ref()
    }

    pub fn buchungsart(&self) -> Buchungsart {
        self.buchungsart
    }
}
