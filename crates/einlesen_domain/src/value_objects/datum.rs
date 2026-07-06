use crate::fehler::ZeilenError;

/// Ein einzelner Kalendertag als reine Wert-Bestandteile (Jahr/Monat/Tag) —
/// **dependency-frei** getragen (die Domäne bindet kein `chrono`). Die *feine*
/// Kalender-Gültigkeit (dass es keinen 30. Februar gibt) garantiert der `Kalender`-Port,
/// hinter dem in `io` `chrono` steckt; dieses Wert-Objekt prüft im Konstruktor nur die
/// groben Feld-Bereiche (Monat `1..=12`, Tag `1..=31`) und trägt danach den Beweis.
/// Jahr/Monat/Tag geben Zahlen heraus, weil die spätere Rhythmus-Erkennung (`auswerten`)
/// auf Monat und Jahr rechnet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kalendertag {
    jahr: u16,
    monat: u8,
    tag: u8,
}

impl Kalendertag {
    /// Baut einen Kalendertag aus seinen Bestandteilen — der Konstruktor, den der
    /// `Kalender`-Port (in `io`, mit `chrono`) nach dem Deuten aufruft. Grobe
    /// Bereichsprüfung; ein unmöglicher Monat/Tag ergibt **kein** Datum (`DatumUngültig`).
    pub fn aus_bestandteilen(jahr: u16, monat: u8, tag: u8) -> Result<Self, ZeilenError> {
        if (1..=12).contains(&monat) && (1..=31).contains(&tag) {
            Ok(Self { jahr, monat, tag })
        } else {
            Err(ZeilenError::DatumUngültig(format!(
                "{tag:02}.{monat:02}.{jahr}"
            )))
        }
    }

    pub fn jahr(&self) -> u16 {
        self.jahr
    }

    pub fn monat(&self) -> u8 {
        self.monat
    }

    pub fn tag(&self) -> u8 {
        self.tag
    }
}

/// Die zeitliche Verortung einer Buchung — sie trägt **zwei** Kalendertage: den
/// `buchungstag` (wann die Bank gebucht hat) und die `valuta` (Wertstellung). Beide
/// beschreiben das „Wann" der Buchung und gehören zusammen, deshalb ein gemeinsames
/// Wert-Objekt statt zweier loser Felder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Datum {
    buchungstag: Kalendertag,
    valuta: Kalendertag,
}

impl Datum {
    /// Bündelt Buchungstag und Valuta zum Datum der Buchung. Beide sind bereits gültige
    /// Kalendertage — hier gibt es nichts mehr abzuweisen.
    pub fn aus_tagen(buchungstag: Kalendertag, valuta: Kalendertag) -> Self {
        Self {
            buchungstag,
            valuta,
        }
    }

    pub fn buchungstag(&self) -> Kalendertag {
        self.buchungstag
    }

    pub fn valuta(&self) -> Kalendertag {
        self.valuta
    }
}

#[cfg(test)]
mod tests {
    use super::Kalendertag;
    use crate::fehler::ZeilenError;

    #[test]
    fn gültige_bestandteile_ergeben_einen_kalendertag() {
        let tag = Kalendertag::aus_bestandteilen(2024, 6, 13).expect("gültig");
        assert_eq!((tag.jahr(), tag.monat(), tag.tag()), (2024, 6, 13));
    }

    #[test]
    fn monat_ausserhalb_eins_bis_zwölf_wird_abgelehnt() {
        assert_eq!(
            Kalendertag::aus_bestandteilen(2024, 13, 1),
            Err(ZeilenError::DatumUngültig("01.13.2024".to_string()))
        );
    }

    #[test]
    fn tag_null_wird_abgelehnt() {
        assert_eq!(
            Kalendertag::aus_bestandteilen(2024, 6, 0),
            Err(ZeilenError::DatumUngültig("00.06.2024".to_string()))
        );
    }
}
