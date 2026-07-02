use std::str::FromStr;

/// Das UBQL-DTO für die zu obfuskierende IBAN am Slice-Rand — der **Eingang** der
/// Obfuskierung. Selbstvalidierend: [`Iban::new`]/`from_str` prüfen Struktur und
/// mod-97-Prüfsumme und normalisieren (Großbuchstaben, ohne Leerzeichen), damit
/// verschiedene Schreibweisen desselben Kontos dasselbe `KONTO_x` erhalten.
///
/// Struktureller Identifikator: leitet **kein** `Serialize`/`Deserialize` ab (der
/// compilergeprüfte `assert_not_impl_any!`-Beleg liegt in `lib.rs`) — eine IBAN
/// geht in den Slice hinein, verlässt die Maschine aber nie
/// ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
///
/// Die IBAN-Validierung ist hier **slice-eigen** (kein geteilter Typ mit
/// `eigene_konten`): Obfuskierung deutet keine Kontonummer, sie braucht nur einen
/// stabilen, normalisierten Schlüssel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iban(String);

impl Iban {
    /// Baut das DTO aus rohem Text — validiert (Struktur + mod-97) und
    /// normalisiert. Liefert bei Verstoß den konkreten Grund.
    pub fn new(input: impl AsRef<str>) -> Result<Self, IbanError> {
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

    /// Die normalisierte IBAN (Großbuchstaben, ohne Leerzeichen) — der First-Seen-
    /// Schlüssel, den die Domäne obfuskiert.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Iban {
    type Err = IbanError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::new(input)
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
/// Fehlermeldung getrennt.
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
