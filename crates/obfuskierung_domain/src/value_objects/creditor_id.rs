use crate::value_objects::non_empty_string::NonEmptyString;

/// Kürzeste denkbare Gläubiger-ID: Land (2) + Prüfziffern (2) + Geschäftsbereich (3) + mindestens
/// ein Zeichen nationale Kennung.
const MIN_LENGTH: usize = 8;
/// Längste Gläubiger-ID laut SEPA-Regelwerk.
const MAX_LENGTH: usize = 35;
/// Der Teiler der ISO-7064-Prüfung (Mod 97-10), dieselbe Rechnung wie bei der IBAN.
const MODULUS: u32 = 97;

/// Die **Gläubiger-Identifikation** (SEPA Creditor ID) einer Firma — der eine strukturelle
/// Identifikator, der die Maschine verlassen darf: eine öffentliche Firmenkennung, bewusst als
/// Analyse-Anker bewahrt (CONTEXT.md, [ADR-0001](../../../docs/adr/0001-hexagonale-architektur.md)).
/// Wird **nicht** obfuskiert.
///
/// **Geparst, nicht geglaubt** (ADR-0005, Issue 0011): weil sie unverändert in den Fingerabdruck und
/// damit in den Chat-Export geht, darf in ihr nur stehen, was **die Form** einer Gläubiger-ID hat —
/// zwei Buchstaben Land, zwei Prüfziffern, drei Zeichen Geschäftsbereich, dann die nationale Kennung,
/// und die Prüfziffern stimmen (Mod 97 über Kennung + Land + Prüfziffern, **ohne** den
/// Geschäftsbereich). Eine IBAN, ein Name oder anderer Unsinn in der Spalte ist **keine** Gläubiger-ID
/// und gilt als **fehlend** — die Gegenpartei wird dann fail-safe als Person gedeutet.
///
/// Erwartet wird die Schreibweise der Bank: Großbuchstaben, ohne Leerzeichen. Eine abweichende
/// Schreibweise fällt fail-safe durch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CreditorId(NonEmptyString);

impl CreditorId {
    /// Parst die rohe Zelle; `None`, wenn sie keine gültige Gläubiger-ID ist.
    #[must_use]
    pub fn parse(raw: NonEmptyString) -> Option<Self> {
        is_valid(raw.as_str()).then_some(Self(raw))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Die **eine** Prüfregel der Gläubiger-ID — öffentlich, damit io bei der Namensliste des Scrubbings
/// dieselbe Grenze zieht wie die Deutung Firma/Person (eine zweite Kopie könnte abweichen).
#[must_use]
pub fn is_valid(raw: &str) -> bool {
    let chars = raw.chars().collect::<Vec<_>>();
    let [c1, c2, d1, d2, b1, b2, b3, national @ ..] = chars.as_slice() else {
        return false;
    };
    let upper_alphanumeric = |c: &char| c.is_ascii_uppercase() || c.is_ascii_digit();
    (MIN_LENGTH..=MAX_LENGTH).contains(&chars.len())
        && c1.is_ascii_uppercase()
        && c2.is_ascii_uppercase()
        && d1.is_ascii_digit()
        && d2.is_ascii_digit()
        && [b1, b2, b3].into_iter().all(upper_alphanumeric)
        && national.iter().all(upper_alphanumeric)
        && remainder(national.iter().chain([c1, c2, d1, d2])) == Some(1)
}

/// Der Rest der ISO-7064-Prüfung: jede Ziffer zählt als sie selbst, jeder Buchstabe als 10–35. Die
/// Rechnung läuft stellenweise, damit keine Zahl überläuft.
fn remainder<'c>(mut chars: impl Iterator<Item = &'c char>) -> Option<u32> {
    chars.try_fold(0_u32, |rest, c| {
        let value = c.to_digit(36)?;
        let shift = if value < 10 { 10 } else { 100 };
        Some((rest * shift + value) % MODULUS)
    })
}
