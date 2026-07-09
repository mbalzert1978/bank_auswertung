use super::{Direction, NonEmptyString, Sign};
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
/// unkonstruierbar (ADR-0005). Betrag und Richtung entstehen **gemeinsam** aus den
/// dekodierten Bestandteilen des Betragsfelds (`with_direction`); die *Zahlenformat*-
/// Dekodierung (deutscher Tausenderpunkt, Dezimalkomma) ist Format-Mechanik und lebt
/// hinter dem `Money`-Port im Adapter/io (ADR-0008), nicht in der Domäne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount {
    cent: u64,
    currency: Currency,
}

impl Amount {
    /// Baut Betrag und Richtung **gemeinsam** aus den vom `Money`-Port dekodierten
    /// Bestandteilen: der vorzeichenlosen Cent-Menge und dem drei-wertigen Vorzeichen.
    /// Das Vorzeichen deutet `Direction::from_sign` (dort lebt seine Bedeutung); fehlt
    /// es, ist die Richtung unbestimmbar (`SignMissing`) — kein stilles `Incoming`. So
    /// lebt das Vorzeichen allein in der Richtung (ADR-0005).
    pub fn with_direction(
        cent: u64,
        sign: Sign,
        currency: Currency,
    ) -> Result<(Self, Direction), LineError> {
        let direction = Direction::from_sign(sign).ok_or(LineError::SignMissing)?;
        Ok((Self { cent, currency }, direction))
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
    use super::{Amount, Currency, Direction, Sign};
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
    fn negatives_vorzeichen_wird_zu_ausgang() {
        let (betrag, richtung) =
            Amount::with_direction(123_456, Sign::Negative, Currency::Euro).expect("deutbar");
        assert_eq!(betrag.cent(), 123_456);
        assert_eq!(betrag.currency(), Currency::Euro);
        assert_eq!(richtung, Direction::Outgoing);
    }

    #[test]
    fn positives_vorzeichen_wird_zu_eingang() {
        let (betrag, richtung) =
            Amount::with_direction(5_000, Sign::Positive, Currency::Euro).expect("deutbar");
        assert_eq!(betrag.cent(), 5_000);
        assert_eq!(richtung, Direction::Incoming);
    }

    #[test]
    fn gleicher_betrag_mit_verschiedenem_vorzeichen_hat_gleichen_cent_wert() {
        let (negativ, _) =
            Amount::with_direction(5_000, Sign::Negative, Currency::Euro).expect("deutbar");
        let (positiv, _) =
            Amount::with_direction(5_000, Sign::Positive, Currency::Euro).expect("deutbar");
        assert_eq!(negativ, positiv);
    }

    #[test]
    fn fehlendes_vorzeichen_wird_als_vorzeichen_fehlt_abgelehnt() {
        assert_eq!(
            Amount::with_direction(5_000, Sign::Missing, Currency::Euro),
            Err(LineError::SignMissing)
        );
    }
}
