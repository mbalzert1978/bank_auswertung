use super::{Direction, NonEmptyString};
use crate::error::LineError;

/// Die Währung eines Betrags. Bewusst ein Summentyp statt eines rohen Codes: nur
/// bekannte Währungen sind konstruierbar. Heute nur `Euro`; weitere kommen hinzu,
/// sobald ein Format sie liefert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Euro,
}

impl Currency {
    /// Deutet einen Währungscode (`EUR`, case-insensitiv). Ein unbekannter Code wird
    /// abgewiesen, statt still falsch weiterzurechnen.
    pub fn from_code(raw: &NonEmptyString) -> Result<Self, LineError> {
        if raw.as_str().eq_ignore_ascii_case("EUR") {
            Ok(Self::Euro)
        } else {
            Err(LineError::CurrencyUnknown)
        }
    }
}

/// Ein Geldbetrag als **vorzeichenlose** Menge in Cent mit Währung — das Vorzeichen
/// lebt allein in der [`Direction`](crate::Direction), ein negativer Betrag ist dadurch
/// unkonstruierbar (ADR-0005). Betrag und Richtung entstehen **gemeinsam** aus dem
/// signierten Betragsfeld (`with_direction`) — die einzige Parse-Autorität für dieses Feld.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount {
    cent: u64,
    currency: Currency,
}

impl Amount {
    /// Deutet das **signierte** deutsche Betragsfeld an *einem* Ort: das führende
    /// Vorzeichen deutet `Direction::from_sign` (dort lebt die Vorzeichen-Grammatik)
    /// und wird abgestreift, die Restziffern (`1.234,56`) ergeben den vorzeichenlosen
    /// Betrag. Ein fehlendes führendes `+`/`-` macht die Richtung unbestimmbar
    /// (`SignMissing`); ungültige Restziffern ergeben `AmountInvalid`. So lebt das
    /// Vorzeichen allein in der Richtung (ADR-0005).
    pub fn with_direction(
        raw: &NonEmptyString,
        currency: Currency,
    ) -> Result<(Self, Direction), LineError> {
        let s = raw.as_str();
        let (direction, digits) = Direction::from_sign(s).ok_or(LineError::SignMissing)?;
        let cent = cent_from_german_digits(digits).ok_or(LineError::AmountInvalid)?;
        Ok((Self { cent, currency }, direction))
    }

    pub fn cent(&self) -> u64 {
        self.cent
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }
}

/// Deutet die **vorzeichenlosen** deutschen Betragsziffern (`1.234,56`) zur Cent-Menge:
/// Tausenderpunkte entfallen, das Komma trennt genau zwei Nachkommastellen (fehlt es,
/// sind es null). Ungültiges ergibt `None`.
fn cent_from_german_digits(digits: &str) -> Option<u64> {
    let normalized = digits.replace('.', "");
    let (euro_part, cent_part) = match normalized.split_once(',') {
        Some((euro, cent)) => (euro, Some(cent)),
        None => (normalized.as_str(), None),
    };
    if euro_part.is_empty() || !euro_part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let euro: u64 = euro_part.parse().ok()?;
    let cent: u64 = match cent_part {
        None => 0,
        Some(decimals) => {
            if decimals.len() != 2 || !decimals.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            decimals.parse().ok()?
        }
    };
    euro.checked_mul(100)?.checked_add(cent)
}

#[cfg(test)]
mod tests {
    use super::{Amount, Currency, Direction};
    use crate::error::LineError;
    use crate::value_objects::NonEmptyString;

    fn roh(wert: &str) -> NonEmptyString {
        NonEmptyString::new(wert).expect("Testwert nicht leer")
    }

    #[test]
    fn eur_wird_case_insensitiv_erkannt() {
        assert_eq!(Currency::from_code(&roh("EUR")), Ok(Currency::Euro));
        assert_eq!(Currency::from_code(&roh("  eur ")), Ok(Currency::Euro));
    }

    #[test]
    fn unbekannte_währung_wird_abgelehnt() {
        assert_eq!(
            Currency::from_code(&roh("USD")),
            Err(LineError::CurrencyUnknown)
        );
    }

    #[test]
    fn negativer_betrag_mit_tausenderpunkt_wird_zu_ausgang_und_cent() {
        let (betrag, richtung) =
            Amount::with_direction(&roh("-1.234,56"), Currency::Euro).expect("gültig");
        assert_eq!(betrag.cent(), 123_456);
        assert_eq!(betrag.currency(), Currency::Euro);
        assert_eq!(richtung, Direction::Outgoing);
    }

    #[test]
    fn positiver_betrag_wird_zu_eingang() {
        let (betrag, richtung) =
            Amount::with_direction(&roh("+50,00"), Currency::Euro).expect("gültig");
        assert_eq!(betrag.cent(), 5_000);
        assert_eq!(richtung, Direction::Incoming);
    }

    #[test]
    fn gleicher_betrag_mit_verschiedenem_vorzeichen_hat_gleichen_cent_wert() {
        let (negativ, _) = Amount::with_direction(&roh("-50,00"), Currency::Euro).expect("gültig");
        let (positiv, _) = Amount::with_direction(&roh("+50,00"), Currency::Euro).expect("gültig");
        assert_eq!(negativ, positiv);
    }

    #[test]
    fn betrag_ohne_nachkommastellen_ist_gültig() {
        let (betrag, _) = Amount::with_direction(&roh("-1000"), Currency::Euro).expect("gültig");
        assert_eq!(betrag.cent(), 100_000);
    }

    #[test]
    fn fehlendes_vorzeichen_wird_als_vorzeichen_fehlt_abgelehnt() {
        assert_eq!(
            Amount::with_direction(&roh("50,00"), Currency::Euro),
            Err(LineError::SignMissing)
        );
    }

    #[test]
    fn fremdes_vorzeichen_wird_als_vorzeichen_fehlt_abgelehnt() {
        assert_eq!(
            Amount::with_direction(&roh("*50,00"), Currency::Euro),
            Err(LineError::SignMissing)
        );
    }

    #[test]
    fn betrag_mit_drei_nachkommastellen_ist_ungültig() {
        assert_eq!(
            Amount::with_direction(&roh("-1,234"), Currency::Euro),
            Err(LineError::AmountInvalid)
        );
    }

    #[test]
    fn nicht_numerischer_betrag_ist_ungültig() {
        assert_eq!(
            Amount::with_direction(&roh("-abc"), Currency::Euro),
            Err(LineError::AmountInvalid)
        );
    }
}
