use super::{NonEmptyString, Richtung};
use crate::fehler::ZeilenError;

/// Die Währung eines Betrags. Bewusst ein Summentyp statt eines rohen Codes: nur
/// bekannte Währungen sind konstruierbar. Heute nur `Euro`; weitere kommen hinzu,
/// sobald ein Format sie liefert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Währung {
    Euro,
}

impl Währung {
    /// Deutet einen Währungscode (`EUR`, case-insensitiv). Ein unbekannter Code wird
    /// abgewiesen, statt still falsch weiterzurechnen.
    pub fn aus_code(roh: &NonEmptyString) -> Result<Self, ZeilenError> {
        if roh.as_str().eq_ignore_ascii_case("EUR") {
            Ok(Self::Euro)
        } else {
            Err(ZeilenError::WährungUnbekannt)
        }
    }
}

/// Ein Geldbetrag als **vorzeichenlose** Menge in Cent mit Währung — das Vorzeichen
/// lebt allein in der [`Richtung`](crate::Richtung), ein negativer Betrag ist dadurch
/// unkonstruierbar (ADR-0005). Betrag und Richtung entstehen **gemeinsam** aus dem
/// signierten Betragsfeld (`mit_richtung`) — die einzige Parse-Autorität für dieses Feld.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Betrag {
    cent: u64,
    währung: Währung,
}

impl Betrag {
    /// Deutet das **signierte** deutsche Betragsfeld an *einem* Ort: das führende
    /// Vorzeichen deutet `Richtung::aus_vorzeichen` (dort lebt die Vorzeichen-Grammatik)
    /// und wird abgestreift, die Restziffern (`1.234,56`) ergeben den vorzeichenlosen
    /// Betrag. Ein fehlendes führendes `+`/`-` macht die Richtung unbestimmbar
    /// (`VorzeichenFehlt`); ungültige Restziffern ergeben `BetragUngültig`. So lebt das
    /// Vorzeichen allein in der Richtung (ADR-0005).
    pub fn mit_richtung(
        roh: &NonEmptyString,
        währung: Währung,
    ) -> Result<(Self, Richtung), ZeilenError> {
        let s = roh.as_str();
        let (richtung, ziffern) =
            Richtung::aus_vorzeichen(s).ok_or(ZeilenError::VorzeichenFehlt)?;
        let cent = cent_aus_deutschen_ziffern(ziffern).ok_or(ZeilenError::BetragUngültig)?;
        Ok((Self { cent, währung }, richtung))
    }

    pub fn cent(&self) -> u64 {
        self.cent
    }

    pub fn währung(&self) -> Währung {
        self.währung
    }
}

/// Deutet die **vorzeichenlosen** deutschen Betragsziffern (`1.234,56`) zur Cent-Menge:
/// Tausenderpunkte entfallen, das Komma trennt genau zwei Nachkommastellen (fehlt es,
/// sind es null). Ungültiges ergibt `None`.
fn cent_aus_deutschen_ziffern(ziffern: &str) -> Option<u64> {
    let normalisiert = ziffern.replace('.', "");
    let (euro_teil, cent_teil) = match normalisiert.split_once(',') {
        Some((euro, cent)) => (euro, Some(cent)),
        None => (normalisiert.as_str(), None),
    };
    if euro_teil.is_empty() || !euro_teil.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let euro: u64 = euro_teil.parse().ok()?;
    let cent: u64 = match cent_teil {
        None => 0,
        Some(nachkomma) => {
            if nachkomma.len() != 2 || !nachkomma.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            nachkomma.parse().ok()?
        }
    };
    euro.checked_mul(100)?.checked_add(cent)
}

#[cfg(test)]
mod tests {
    use super::{Betrag, Richtung, Währung};
    use crate::fehler::ZeilenError;
    use crate::value_objects::NonEmptyString;

    fn roh(wert: &str) -> NonEmptyString {
        NonEmptyString::new(wert).expect("Testwert nicht leer")
    }

    #[test]
    fn eur_wird_case_insensitiv_erkannt() {
        assert_eq!(Währung::aus_code(&roh("EUR")), Ok(Währung::Euro));
        assert_eq!(Währung::aus_code(&roh("  eur ")), Ok(Währung::Euro));
    }

    #[test]
    fn unbekannte_währung_wird_abgelehnt() {
        assert_eq!(
            Währung::aus_code(&roh("USD")),
            Err(ZeilenError::WährungUnbekannt)
        );
    }

    #[test]
    fn negativer_betrag_mit_tausenderpunkt_wird_zu_ausgang_und_cent() {
        let (betrag, richtung) =
            Betrag::mit_richtung(&roh("-1.234,56"), Währung::Euro).expect("gültig");
        assert_eq!(betrag.cent(), 123_456);
        assert_eq!(betrag.währung(), Währung::Euro);
        assert_eq!(richtung, Richtung::Ausgang);
    }

    #[test]
    fn positiver_betrag_wird_zu_eingang() {
        let (betrag, richtung) =
            Betrag::mit_richtung(&roh("+50,00"), Währung::Euro).expect("gültig");
        assert_eq!(betrag.cent(), 5_000);
        assert_eq!(richtung, Richtung::Eingang);
    }

    #[test]
    fn gleicher_betrag_mit_verschiedenem_vorzeichen_hat_gleichen_cent_wert() {
        let (negativ, _) = Betrag::mit_richtung(&roh("-50,00"), Währung::Euro).expect("gültig");
        let (positiv, _) = Betrag::mit_richtung(&roh("+50,00"), Währung::Euro).expect("gültig");
        assert_eq!(negativ, positiv);
    }

    #[test]
    fn betrag_ohne_nachkommastellen_ist_gültig() {
        let (betrag, _) = Betrag::mit_richtung(&roh("-1000"), Währung::Euro).expect("gültig");
        assert_eq!(betrag.cent(), 100_000);
    }

    #[test]
    fn fehlendes_vorzeichen_wird_als_vorzeichen_fehlt_abgelehnt() {
        assert_eq!(
            Betrag::mit_richtung(&roh("50,00"), Währung::Euro),
            Err(ZeilenError::VorzeichenFehlt)
        );
    }

    #[test]
    fn fremdes_vorzeichen_wird_als_vorzeichen_fehlt_abgelehnt() {
        assert_eq!(
            Betrag::mit_richtung(&roh("*50,00"), Währung::Euro),
            Err(ZeilenError::VorzeichenFehlt)
        );
    }

    #[test]
    fn betrag_mit_drei_nachkommastellen_ist_ungültig() {
        assert_eq!(
            Betrag::mit_richtung(&roh("-1,234"), Währung::Euro),
            Err(ZeilenError::BetragUngültig)
        );
    }

    #[test]
    fn nicht_numerischer_betrag_ist_ungültig() {
        assert_eq!(
            Betrag::mit_richtung(&roh("-abc"), Währung::Euro),
            Err(ZeilenError::BetragUngültig)
        );
    }
}
