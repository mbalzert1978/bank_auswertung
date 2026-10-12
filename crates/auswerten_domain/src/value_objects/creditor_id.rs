use crate::value_objects::NonEmptyString;

/// Kürzeste denkbare Gläubiger-ID: Land (2) + Prüfziffern (2) + Geschäftsbereich (3) + mindestens
/// ein Zeichen nationale Kennung.
const MIN_LENGTH: usize = 8;
/// Längste Gläubiger-ID laut SEPA-Regelwerk.
const MAX_LENGTH: usize = 35;
/// Der Teiler der ISO-7064-Prüfung (Mod 97-10), dieselbe Rechnung wie bei der IBAN.
const MODULUS: u32 = 97;

/// Die **Gläubiger-ID** einer Firma, so wie dieser Slice sie führt — **geparst**, nicht geglaubt
/// (Issue 0011, ADR-0005).
///
/// **Warum hier noch einmal.** Sie geht unverändert in den [`Fingerprint`](crate::Fingerprint) und
/// damit in den Chat-Export. `obfuskieren` parst sie schon beim Ablegen — aber jeder Lauf liest die
/// **ganze** `obfuskiert`-Historie, auch Zeilen aus einer Datei eines älteren Programmstands, die
/// diese Prüfung nie gesehen haben. Darum parst die ACL dieses Slice jede Firmen-Zeile an der
/// Persistenzgrenze ein zweites Mal; ist sie keine Gläubiger-ID, endet der Lauf terminal
/// ([`EvaluationError::InvalidCreditorId`](crate::EvaluationError::InvalidCreditorId)).
///
/// **Dieselbe Regel wie in `obfuskieren`:** zwei Buchstaben Land, zwei Prüfziffern, drei Zeichen
/// Geschäftsbereich, dann die nationale Kennung (Großbuchstaben/Ziffern); Mod 97 über Kennung + Land +
/// Prüfziffern, **ohne** den Geschäftsbereich. Die Domänen sind blind füreinander, eine geteilte
/// Funktion gibt es darum nicht — dass beide Kopien dasselbe urteilen, hält ein io-Test über eine
/// Fall-Tabelle fest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CreditorId(NonEmptyString);

impl CreditorId {
    /// Parst den Wert; `None`, wenn er keine gültige Gläubiger-ID ist.
    #[must_use]
    pub fn parse(raw: NonEmptyString) -> Option<Self> {
        is_valid(raw.as_str()).then_some(Self(raw))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Die Prüfregel der Gläubiger-ID (siehe [`CreditorId`]).
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

/// Der Rest der ISO-7064-Prüfung: jede Ziffer zählt als sie selbst, jeder Buchstabe als 10–35.
fn remainder<'c>(mut chars: impl Iterator<Item = &'c char>) -> Option<u32> {
    chars.try_fold(0_u32, |rest, c| {
        let value = c.to_digit(36)?;
        let shift = if value < 10 { 10 } else { 100 };
        Some((rest * shift + value) % MODULUS)
    })
}
