use super::NonEmptyString;
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
        match roh.as_str().to_ascii_uppercase().as_str() {
            "EUR" => Ok(Self::Euro),
            _ => Err(ZeilenError::WährungUnbekannt(roh.as_str().to_string())),
        }
    }
}

/// Ein Geldbetrag als **vorzeichenlose** Menge in Cent mit Währung — das Vorzeichen
/// lebt allein in der [`Richtung`](crate::Richtung), ein negativer Betrag ist dadurch
/// unkonstruierbar (ADR-0005). Parst sich selbst aus dem deutschen Betragsformat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Betrag {
    cent: u64,
    währung: Währung,
}

impl Betrag {
    /// Deutet einen deutschen Geldbetrag (`1.234,56`, optional mit Vorzeichen) zur
    /// vorzeichenlosen Cent-Menge: das Vorzeichen wird verworfen (es lebt in der
    /// Richtung), Tausenderpunkte entfallen, das Komma trennt genau zwei
    /// Nachkommastellen. Ungültiges Format wird abgewiesen.
    pub fn aus_deutschem_betrag(
        roh: &NonEmptyString,
        währung: Währung,
    ) -> Result<Self, ZeilenError> {
        let s = roh.as_str();
        let ungültig = || ZeilenError::BetragUngültig(s.to_string());
        let ohne_vorzeichen = s
            .strip_prefix('-')
            .or_else(|| s.strip_prefix('+'))
            .unwrap_or(s);
        let normalisiert = ohne_vorzeichen.replace('.', "");
        let mut teile = normalisiert.split(',');
        let euro_teil = teile.next().unwrap_or_default();
        let cent_teil = teile.next();
        if teile.next().is_some() {
            return Err(ungültig());
        }
        if euro_teil.is_empty() || !euro_teil.bytes().all(|b| b.is_ascii_digit()) {
            return Err(ungültig());
        }
        let euro: u64 = euro_teil.parse().ok().ok_or_else(ungültig)?;
        let cent: u64 = match cent_teil {
            None => 0,
            Some(nachkomma) => {
                if nachkomma.len() != 2 || !nachkomma.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(ungültig());
                }
                nachkomma.parse().ok().ok_or_else(ungültig)?
            }
        };
        let gesamt = euro
            .checked_mul(100)
            .and_then(|betrag| betrag.checked_add(cent))
            .ok_or_else(ungültig)?;
        Ok(Self {
            cent: gesamt,
            währung,
        })
    }

    pub fn cent(&self) -> u64 {
        self.cent
    }

    pub fn währung(&self) -> Währung {
        self.währung
    }
}

#[cfg(test)]
mod tests {
    use super::{Betrag, Währung};
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
            Err(ZeilenError::WährungUnbekannt("USD".to_string()))
        );
    }

    #[test]
    fn deutscher_betrag_mit_tausenderpunkt_wird_zu_cent() {
        let betrag = Betrag::aus_deutschem_betrag(&roh("1.234,56"), Währung::Euro).expect("gültig");
        assert_eq!(betrag.cent(), 123_456);
        assert_eq!(betrag.währung(), Währung::Euro);
    }

    #[test]
    fn vorzeichen_wird_verworfen_betrag_bleibt_vorzeichenlos() {
        let negativ = Betrag::aus_deutschem_betrag(&roh("-50,00"), Währung::Euro).expect("gültig");
        let positiv = Betrag::aus_deutschem_betrag(&roh("+50,00"), Währung::Euro).expect("gültig");
        assert_eq!(negativ.cent(), 5_000);
        assert_eq!(negativ, positiv);
    }

    #[test]
    fn betrag_ohne_nachkommastellen_ist_gültig() {
        let betrag = Betrag::aus_deutschem_betrag(&roh("1000"), Währung::Euro).expect("gültig");
        assert_eq!(betrag.cent(), 100_000);
    }

    #[test]
    fn betrag_mit_drei_nachkommastellen_ist_ungültig() {
        assert_eq!(
            Betrag::aus_deutschem_betrag(&roh("1,234"), Währung::Euro),
            Err(ZeilenError::BetragUngültig("1,234".to_string()))
        );
    }

    #[test]
    fn nicht_numerischer_betrag_ist_ungültig() {
        assert_eq!(
            Betrag::aus_deutschem_betrag(&roh("abc"), Währung::Euro),
            Err(ZeilenError::BetragUngültig("abc".to_string()))
        );
    }
}
