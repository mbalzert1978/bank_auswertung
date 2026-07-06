use crate::fehler::ZeilenError;
use crate::ports::{Kalender, Rohzeile};
use crate::value_objects::{
    Betrag, Buchungstext, Datum, Empfänger, Iban, Richtung, Verwendungszweck, Währung,
};

/// Eine eingelesene, validierte Buchungszeile — das Aggregat des Slice. Es entsteht
/// **ausschließlich** über `aus_rohzeile`, das die Rohdaten (Port `Rohzeile`) an *einem*
/// Ort parst und die Pflicht/Optional-Entscheidung je Feld trifft. Danach trägt es nur
/// noch gültige Wert-Objekte; rohe Strings gibt es hier nicht mehr. Die Getter liest der
/// Adapter-Handler aus, um daraus das ausgehende `Eingelesen`-Command zu bauen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZeilenAggregat {
    auftragskonto: Iban,
    gegenkonto: Option<Iban>,
    betrag: Betrag,
    richtung: Richtung,
    empfänger: Empfänger,
    datum: Datum,
    verwendungszweck: Option<Verwendungszweck>,
    buchungstext: Option<Buchungstext>,
}

impl ZeilenAggregat {
    /// Deutet eine Rohzeile zu einem Aggregat — die **einzige** Konstruktion. Zuerst die
    /// Pflichtwerte (Währung, Betrag/Richtung aus demselben Betragsfeld, Datum aus
    /// Buchungstag + Valuta über den Kalender, Auftragskonto, Empfänger), dann die
    /// optionalen (Gegenkonto, Verwendungszweck, Buchungstext). Jeder Fehler ist terminal.
    pub fn aus_rohzeile(
        zeile: &dyn Rohzeile,
        kalender: &dyn Kalender,
    ) -> Result<Self, ZeilenError> {
        let währung = Währung::aus_code(&zeile.währung().ok_or(ZeilenError::WährungFehlt)?)?;
        let roh_betrag = zeile.betrag().ok_or(ZeilenError::BetragFehlt)?;
        let betrag = Betrag::aus_deutschem_betrag(&roh_betrag, währung)?;
        let richtung = Richtung::aus_deutschem_betrag(&roh_betrag)?;
        let buchungstag = kalender.deute(&zeile.buchungstag().ok_or(ZeilenError::DatumFehlt)?)?;
        let valuta = kalender.deute(&zeile.valuta().ok_or(ZeilenError::DatumFehlt)?)?;
        let datum = Datum::aus_tagen(buchungstag, valuta);
        let auftragskonto = zeile
            .auftragskonto()
            .map(Iban::from)
            .ok_or(ZeilenError::AuftragskontoFehlt)?;
        let gegenkonto = zeile.gegenkonto().map(Iban::from);
        let empfänger = Empfänger::aus_teilen(zeile.empfänger_name(), zeile.gläubiger_id())?;
        let verwendungszweck = zeile.verwendungszweck().map(Verwendungszweck::from);
        let buchungstext = zeile.buchungstext().map(Buchungstext::from);
        Ok(Self {
            auftragskonto,
            gegenkonto,
            betrag,
            richtung,
            empfänger,
            datum,
            verwendungszweck,
            buchungstext,
        })
    }

    pub fn auftragskonto(&self) -> &Iban {
        &self.auftragskonto
    }

    pub fn gegenkonto(&self) -> Option<&Iban> {
        self.gegenkonto.as_ref()
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

    pub fn datum(&self) -> Datum {
        self.datum
    }

    pub fn verwendungszweck(&self) -> Option<&Verwendungszweck> {
        self.verwendungszweck.as_ref()
    }

    pub fn buchungstext(&self) -> Option<&Buchungstext> {
        self.buchungstext.as_ref()
    }
}
