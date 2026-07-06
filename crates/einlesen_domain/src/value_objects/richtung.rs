use super::NonEmptyString;
use crate::fehler::ZeilenError;

/// Die Richtung eines Umsatzes — der einzige Ort, an dem das Vorzeichen lebt
/// (ADR-0005), sodass ein negativer [`Betrag`](crate::Betrag) unkonstruierbar ist.
/// Wird aus dem Vorzeichen des deutschen Betragsfelds gedeutet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Richtung {
    Eingang,
    Ausgang,
}

impl Richtung {
    /// Liest die Richtung aus dem **Vorzeichen** des rohen Betragsfelds: führendes
    /// `-` → `Ausgang`, führendes `+` → `Eingang`. Jedes **andere** erste Zeichen ist
    /// ein Formatfehler und wird abgewiesen — kein stilles `Eingang`. Das Betragsfeld
    /// trägt genau eins von beiden Vorzeichen; ein fehlendes (führende Ziffer) oder
    /// fremdes (`*`, Buchstabe) Zeichen ist ungültig.
    pub fn aus_deutschem_betrag(roh: &NonEmptyString) -> Result<Self, ZeilenError> {
        match roh.as_str().chars().next() {
            Some('-') => Ok(Self::Ausgang),
            Some('+') => Ok(Self::Eingang),
            _ => Err(ZeilenError::BetragUngültig(roh.as_str().to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Richtung;
    use crate::fehler::ZeilenError;
    use crate::value_objects::NonEmptyString;

    fn roh(wert: &str) -> NonEmptyString {
        NonEmptyString::new(wert).expect("Testwert nicht leer")
    }

    #[test]
    fn führendes_minus_ist_ausgang() {
        assert_eq!(
            Richtung::aus_deutschem_betrag(&roh("-50,00")),
            Ok(Richtung::Ausgang)
        );
    }

    #[test]
    fn führendes_plus_ist_eingang() {
        assert_eq!(
            Richtung::aus_deutschem_betrag(&roh("+50,00")),
            Ok(Richtung::Eingang)
        );
    }

    #[test]
    fn fehlendes_vorzeichen_wird_abgelehnt() {
        assert_eq!(
            Richtung::aus_deutschem_betrag(&roh("50,00")),
            Err(ZeilenError::BetragUngültig("50,00".to_string()))
        );
    }

    #[test]
    fn fremdes_vorzeichen_wird_abgelehnt() {
        assert_eq!(
            Richtung::aus_deutschem_betrag(&roh("*50,00")),
            Err(ZeilenError::BetragUngültig("*50,00".to_string()))
        );
    }
}
