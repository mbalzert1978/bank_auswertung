/// Eine strukturell und per Prüfsumme (ISO 7064, mod 97-10) validierte IBAN —
/// der Domänen-Wert des Eigene-Konten-Slice.
///
/// Struktureller Identifikator: die Domäne bindet **kein** serde, also kann diese
/// IBAN gar kein `Serialize`/`Deserialize` ableiten und ist in kein DTO
/// einbettbar — eine eigene IBAN darf die Maschine nie verlassen
/// ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md); der
/// compilergeprüfte `assert_not_impl_any!`-Beleg liegt am Rand, im Adapter-DTO
/// `Iban`). Konstruktion nur über den validierenden [`IbanEntity::parse`] (parse,
/// don't validate). Gespeichert wird die normalisierte Form (Großbuchstaben, ohne
/// Leerzeichen), damit Vergleich und Nachschlagen schreibweise-unabhängig sind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IbanEntity(String);

impl IbanEntity {
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
/// Fehlermeldung getrennt. Von Hand ausgegeben (die Domäne bindet keine
/// Fehler-Bibliothek); der Adapter mappt/umschließt ihn nach Belieben.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IbanError {
    /// Länge außerhalb 15–34 Zeichen.
    Length(usize),
    /// Enthält Zeichen außerhalb von A–Z und 0–9.
    InvalidChar,
    /// Beginnt nicht mit zwei Buchstaben und zwei Prüfziffern.
    Structure,
    /// mod-97-Prüfsumme stimmt nicht.
    Checksum,
}

impl std::fmt::Display for IbanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Length(count) => write!(f, "IBAN hat {count} Zeichen, erlaubt sind 15–34"),
            Self::InvalidChar => f.write_str("IBAN enthält unerlaubte Zeichen (nur A–Z, 0–9)"),
            Self::Structure => {
                f.write_str("IBAN beginnt nicht mit zwei Buchstaben und zwei Prüfziffern")
            }
            Self::Checksum => f.write_str("IBAN-Prüfsumme (mod 97) stimmt nicht"),
        }
    }
}

impl std::error::Error for IbanError {}
