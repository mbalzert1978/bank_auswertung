use super::{DecodedAmount, Direction, NonEmptyString};
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
/// unkonstruierbar (ADR-0005). Betrag und Richtung entstehen **gemeinsam** aus dem vom
/// `Money`-Port dekodierten [`DecodedAmount`](crate::DecodedAmount); die *Zahlenformat*-
/// Dekodierung (deutscher Tausenderpunkt, Dezimalkomma) ist Format-Mechanik und lebt
/// hinter dem `Money`-Port im Adapter/io (ADR-0008), nicht in der Domäne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount {
    cent: u64,
    currency: Currency,
}

impl Amount {
    /// Baut Betrag und Richtung **gemeinsam** aus dem vom `Money`-Port dekodierten
    /// [`DecodedAmount`](crate::DecodedAmount): die vorzeichenlose Cent-Menge wird zum
    /// Betrag, das zwei-wertige Vorzeichen deutet `Direction::from_sign` (dort lebt seine
    /// Bedeutung). **Unfehlbar** — ein nicht dekodierbares Feld hat den Port schon als
    /// `None` verlassen (im Aggregat `AmountInvalid`); hier gibt es nichts mehr
    /// abzuweisen. So lebt das Vorzeichen allein in der Richtung (ADR-0005).
    pub fn with_direction(decoded: DecodedAmount, currency: Currency) -> (Self, Direction) {
        let direction = Direction::from_sign(decoded.sign());
        (
            Self {
                cent: decoded.cent(),
                currency,
            },
            direction,
        )
    }

    pub fn cent(&self) -> u64 {
        self.cent
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }
}

#[cfg(test)]
mod tests {
    use super::{Amount, Currency, Direction};
    use crate::error::LineError;
    use crate::value_objects::{DecodedAmount, NonEmptyString, Sign};

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
    fn negatives_vorzeichen_wird_zu_ausgang() {
        let (betrag, richtung) =
            Amount::with_direction(DecodedAmount::new(123_456, Sign::Negative), Currency::Euro);
        assert_eq!(betrag.cent(), 123_456);
        assert_eq!(betrag.currency(), Currency::Euro);
        assert_eq!(richtung, Direction::Outgoing);
    }

    #[test]
    fn positives_vorzeichen_wird_zu_eingang() {
        let (betrag, richtung) =
            Amount::with_direction(DecodedAmount::new(5_000, Sign::Positive), Currency::Euro);
        assert_eq!(betrag.cent(), 5_000);
        assert_eq!(richtung, Direction::Incoming);
    }

    #[test]
    fn gleicher_betrag_mit_verschiedenem_vorzeichen_hat_gleichen_cent_wert() {
        let (negativ, _) =
            Amount::with_direction(DecodedAmount::new(5_000, Sign::Negative), Currency::Euro);
        let (positiv, _) =
            Amount::with_direction(DecodedAmount::new(5_000, Sign::Positive), Currency::Euro);
        assert_eq!(negativ, positiv);
    }
}
