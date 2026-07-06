use crate::fehler::ZeilenError;
use crate::ports::{Kalender, Rohzeile};
use crate::value_objects::{
    Betrag, Buchungstext, Datum, Empfänger, Empfängername, GläubigerId, Iban, Kalendertag,
    NonEmptyString, Richtung, Verwendungszweck, Währung,
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
        let (betrag, richtung) = Betrag::mit_richtung(&roh_betrag, währung)?;
        let buchungstag = deute_datum(kalender, zeile.buchungstag())?;
        let valuta = deute_datum(kalender, zeile.valuta())?;
        let datum = Datum::aus_tagen(buchungstag, valuta);
        let auftragskonto = zeile
            .auftragskonto()
            .map(Iban::from)
            .ok_or(ZeilenError::AuftragskontoFehlt)?;
        let gegenkonto = zeile.gegenkonto().map(Iban::from);
        let empfänger = Empfänger::aus_teilen(
            zeile.empfänger_name().map(Empfängername::from),
            zeile.gläubiger_id().map(GläubigerId::from),
        )?;
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

/// Deutet ein rohes Datumsfeld über den `Kalender`-Port und mappt an *einem* Ort auf die
/// terminalen Datumsfehler: ein fehlendes Feld → `DatumFehlt`, ein unlesbares Datum →
/// `DatumUngültig`. Der Port selbst trägt nur `Option` — die Slice-Fehlbedeutung entsteht
/// hier, wo der rohe Wert noch vorliegt.
fn deute_datum(
    kalender: &dyn Kalender,
    roh: Option<NonEmptyString>,
) -> Result<Kalendertag, ZeilenError> {
    let roh = roh.ok_or(ZeilenError::DatumFehlt)?;
    kalender
        .deute(&roh)
        .ok_or_else(|| ZeilenError::DatumUngültig(roh.as_str().to_string()))
}

#[cfg(test)]
mod tests {
    use super::ZeilenAggregat;
    use crate::fehler::ZeilenError;
    use crate::ports::{Kalender, Rohzeile};
    use crate::value_objects::{
        Buchungstext, GläubigerId, Iban, Kalendertag, NonEmptyString, Richtung, Verwendungszweck,
    };

    fn text(wert: &str) -> Option<NonEmptyString> {
        NonEmptyString::new(wert)
    }

    /// Eine vollständig gültige Rohzeile; ein Test überschreibt einzelne Felder auf
    /// `None`, um genau deren Pflicht-/Optional-Verhalten zu isolieren.
    struct FakeZeile {
        auftragskonto: Option<NonEmptyString>,
        buchungstag: Option<NonEmptyString>,
        valuta: Option<NonEmptyString>,
        betrag: Option<NonEmptyString>,
        währung: Option<NonEmptyString>,
        empfänger_name: Option<NonEmptyString>,
        gläubiger_id: Option<NonEmptyString>,
        gegenkonto: Option<NonEmptyString>,
        verwendungszweck: Option<NonEmptyString>,
        buchungstext: Option<NonEmptyString>,
    }

    impl FakeZeile {
        fn gültig() -> Self {
            Self {
                auftragskonto: text("DE89370400440532013000"),
                buchungstag: text("13.06.2024"),
                valuta: text("14.06.2024"),
                betrag: text("-50,00"),
                währung: text("EUR"),
                empfänger_name: text("EDEKA SAGT DANKE"),
                gläubiger_id: text("DE98ZZZ09999999999"),
                gegenkonto: text("DE11520513735120710131"),
                verwendungszweck: text("Einkauf"),
                buchungstext: text("LASTSCHRIFT"),
            }
        }
    }

    impl Rohzeile for FakeZeile {
        fn auftragskonto(&self) -> Option<NonEmptyString> {
            self.auftragskonto.clone()
        }
        fn buchungstag(&self) -> Option<NonEmptyString> {
            self.buchungstag.clone()
        }
        fn valuta(&self) -> Option<NonEmptyString> {
            self.valuta.clone()
        }
        fn betrag(&self) -> Option<NonEmptyString> {
            self.betrag.clone()
        }
        fn währung(&self) -> Option<NonEmptyString> {
            self.währung.clone()
        }
        fn empfänger_name(&self) -> Option<NonEmptyString> {
            self.empfänger_name.clone()
        }
        fn gläubiger_id(&self) -> Option<NonEmptyString> {
            self.gläubiger_id.clone()
        }
        fn gegenkonto(&self) -> Option<NonEmptyString> {
            self.gegenkonto.clone()
        }
        fn verwendungszweck(&self) -> Option<NonEmptyString> {
            self.verwendungszweck.clone()
        }
        fn buchungstext(&self) -> Option<NonEmptyString> {
            self.buchungstext.clone()
        }
    }

    /// Deterministischer Kalender: deutet „TT.MM.JJJJ", sodass Buchungstag und Valuta
    /// im Aggregat unterscheidbar bleiben. Ungültiges Format wird abgewiesen.
    struct DeutscherKalender;

    impl Kalender for DeutscherKalender {
        fn deute(&self, roh: &NonEmptyString) -> Option<Kalendertag> {
            let mut teile = roh.as_str().split('.');
            let tag: u8 = teile.next()?.parse().ok()?;
            let monat: u8 = teile.next()?.parse().ok()?;
            let jahr: u16 = teile.next()?.parse().ok()?;
            Kalendertag::aus_bestandteilen(jahr, monat, tag)
        }
    }

    fn deute(zeile: &FakeZeile) -> Result<ZeilenAggregat, ZeilenError> {
        ZeilenAggregat::aus_rohzeile(zeile, &DeutscherKalender)
    }

    #[test]
    fn alle_felder_gesetzt_ergeben_ein_aggregat_mit_korrekten_gettern() {
        let aggregat = deute(&FakeZeile::gültig()).expect("gültige Zeile");
        assert_eq!(aggregat.auftragskonto().as_str(), "DE89370400440532013000");
        assert_eq!(
            aggregat.gegenkonto().map(Iban::as_str),
            Some("DE11520513735120710131")
        );
        assert_eq!(aggregat.betrag().cent(), 5_000);
        assert_eq!(aggregat.richtung(), Richtung::Ausgang);
        assert_eq!(aggregat.empfänger().name().as_str(), "EDEKA SAGT DANKE");
        assert_eq!(
            aggregat.empfänger().gläubiger_id().map(GläubigerId::as_str),
            Some("DE98ZZZ09999999999")
        );
        assert_eq!(
            aggregat.datum().buchungstag(),
            Kalendertag::aus_bestandteilen(2024, 6, 13).expect("gültig")
        );
        assert_eq!(
            aggregat.datum().valuta(),
            Kalendertag::aus_bestandteilen(2024, 6, 14).expect("gültig")
        );
        assert_eq!(
            aggregat.verwendungszweck().map(Verwendungszweck::as_str),
            Some("Einkauf")
        );
        assert_eq!(
            aggregat.buchungstext().map(Buchungstext::as_str),
            Some("LASTSCHRIFT")
        );
    }

    #[test]
    fn fehlende_währung_wird_terminal_abgelehnt() {
        let mut zeile = FakeZeile::gültig();
        zeile.währung = None;
        assert_eq!(deute(&zeile), Err(ZeilenError::WährungFehlt));
    }

    #[test]
    fn fehlender_betrag_wird_terminal_abgelehnt() {
        let mut zeile = FakeZeile::gültig();
        zeile.betrag = None;
        assert_eq!(deute(&zeile), Err(ZeilenError::BetragFehlt));
    }

    #[test]
    fn fehlendes_auftragskonto_wird_terminal_abgelehnt() {
        let mut zeile = FakeZeile::gültig();
        zeile.auftragskonto = None;
        assert_eq!(deute(&zeile), Err(ZeilenError::AuftragskontoFehlt));
    }

    #[test]
    fn fehlender_empfängername_wird_terminal_abgelehnt() {
        let mut zeile = FakeZeile::gültig();
        zeile.empfänger_name = None;
        assert_eq!(deute(&zeile), Err(ZeilenError::EmpfängerFehlt));
    }

    #[test]
    fn fehlender_buchungstag_wird_als_datum_fehlt_abgelehnt() {
        let mut zeile = FakeZeile::gültig();
        zeile.buchungstag = None;
        assert_eq!(deute(&zeile), Err(ZeilenError::DatumFehlt));
    }

    #[test]
    fn fehlende_valuta_wird_als_datum_fehlt_abgelehnt() {
        let mut zeile = FakeZeile::gültig();
        zeile.valuta = None;
        assert_eq!(deute(&zeile), Err(ZeilenError::DatumFehlt));
    }

    #[test]
    fn fehlende_optionalfelder_ergeben_leere_getter() {
        let mut zeile = FakeZeile::gültig();
        zeile.gegenkonto = None;
        zeile.verwendungszweck = None;
        zeile.buchungstext = None;
        let aggregat = deute(&zeile).expect("Pflichtfelder gesetzt");
        assert!(aggregat.gegenkonto().is_none());
        assert!(aggregat.verwendungszweck().is_none());
        assert!(aggregat.buchungstext().is_none());
    }

    #[test]
    fn vorzeichenloser_betrag_wird_wegen_fehlendem_vorzeichen_verworfen() {
        // „50,00" ist als Betrag wohlgeformt, aber ohne Vorzeichen ist die Richtung
        // nicht deutbar — die Zeile wird terminal verworfen (nicht als BetragUngültig).
        let mut zeile = FakeZeile::gültig();
        zeile.betrag = text("50,00");
        assert_eq!(deute(&zeile), Err(ZeilenError::VorzeichenFehlt));
    }
}
