/// Eine strukturell und per Prüfsumme (ISO 7064, mod 97-10) validierte IBAN.
///
/// Adapter-seitiger struktureller Identifikator: leitet **kein** `Serialize` ab
/// und ist damit in kein DTO einbettbar — eine eigene IBAN darf die Maschine nie
/// verlassen (ADR-0001/0003). `Deserialize` läuft über den validierenden
/// Konstruktor (`serde(try_from = "String")`), sodass auch beim Einlesen aus der
/// Config nur eine geprüfte IBAN entsteht (parse, don't validate).
///
/// Gespeichert wird die normalisierte Form (Großbuchstaben, ohne Leerzeichen),
/// damit Vergleich und Nachschlagen unabhängig von der Schreibweise sind.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct Iban(String);

impl Iban {
    /// Prüft Länge (15–34), Aufbau (zwei Buchstaben Länderkürzel, zwei
    /// Prüfziffern) und die mod-97-Prüfsumme in einem Durchgang. Liefert bei
    /// Verstoß den konkreten Grund, nicht bloß „ungültig".
    pub fn parse(input: impl AsRef<str>) -> Result<Self, IbanError> {
        let normalized: String = input
            .as_ref()
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_uppercase)
            .collect();

        let length = normalized.chars().count();
        if !(15..=34).contains(&length) {
            return Err(IbanError::Length(length));
        }
        let country_code_ok = normalized.chars().take(2).all(|c| c.is_ascii_uppercase());
        let check_digits_ok = normalized
            .chars()
            .skip(2)
            .take(2)
            .all(|c| c.is_ascii_digit());
        if !country_code_ok || !check_digits_ok {
            return Err(IbanError::Structure);
        }
        match mod_97(&normalized) {
            Some(1) => Ok(Self(normalized)),
            Some(_) => Err(IbanError::Checksum),
            None => Err(IbanError::InvalidChar),
        }
    }

    /// Die normalisierte IBAN (Großbuchstaben, ohne Leerzeichen).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Iban {
    type Error = IbanError;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        Self::parse(input)
    }
}

/// mod 97-10 (ISO 7064): die ersten vier Zeichen ans Ende stellen, jeden
/// Buchstaben zu zwei Ziffern ausklappen (A=10 … Z=35) und den Rest stückweise
/// modulo 97 rechnen. `None`, sobald ein Zeichen nicht alphanumerisch ist; eine
/// gültige IBAN ergibt `Some(1)`.
fn mod_97(iban: &str) -> Option<u32> {
    iban.chars()
        .skip(4)
        .chain(iban.chars().take(4))
        .try_fold(0_u32, |remainder, c| {
            c.to_digit(36).map(|value| {
                if value >= 10 {
                    (remainder * 100 + value) % 97
                } else {
                    (remainder * 10 + value) % 97
                }
            })
        })
}

/// Warum eine IBAN abgelehnt wurde — die Ursachen sind für eine gute
/// Fehlermeldung getrennt, auch wenn die Config sie alle gleich behandelt.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IbanError {
    /// Länge außerhalb 15–34 Zeichen.
    #[error("IBAN hat {0} Zeichen, erlaubt sind 15–34")]
    Length(usize),
    /// Enthält Zeichen außerhalb von A–Z und 0–9.
    #[error("IBAN enthält unerlaubte Zeichen (nur A–Z, 0–9)")]
    InvalidChar,
    /// Beginnt nicht mit zwei Buchstaben und zwei Prüfziffern.
    #[error("IBAN beginnt nicht mit zwei Buchstaben und zwei Prüfziffern")]
    Structure,
    /// mod-97-Prüfsumme stimmt nicht.
    #[error("IBAN-Prüfsumme (mod 97) stimmt nicht")]
    Checksum,
}

#[cfg(test)]
mod tests {
    use super::Iban;
    use serde::Serialize;
    use static_assertions::assert_not_impl_any;

    // Datenschutz-Invariant zur Compile-Zeit: die echte IBAN leitet kein
    // `Serialize` ab und ist damit in kein DTO einbettbar — eine eigene IBAN
    // kann die Maschine nicht verlassen. Nur `Deserialize` (beim Einlesen aus
    // der Config, über den validierenden `try_from`) ist zulässig.
    assert_not_impl_any!(Iban: Serialize);
}
