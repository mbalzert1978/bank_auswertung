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
    pub(crate) fn from_code(raw: &NonEmptyString) -> Result<Self, LineError> {
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
    #[must_use]
    pub(crate) fn with_direction(decoded: DecodedAmount, currency: Currency) -> (Self, Direction) {
        let direction = Direction::from_sign(decoded.sign());
        (
            Self {
                cent: decoded.cent(),
                currency,
            },
            direction,
        )
    }

    #[must_use]
    pub fn cent(&self) -> u64 {
        self.cent
    }

    #[must_use]
    pub fn currency(&self) -> Currency {
        self.currency
    }
}
