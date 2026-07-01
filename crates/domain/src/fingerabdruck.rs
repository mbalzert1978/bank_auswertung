use crate::{Buchung, NonEmptyString};

/// Ein stabiler Schlüssel, der eine Art von Buchung für die Kategorisierung
/// identifiziert. Basisversion (Deduplizierung gleichartiger Buchungen über
/// die Empfänger-Identität) — die Zahlungsintermediär-Erkennung (PayPal/
/// Klarna/Amazon Payments als Durchleiter statt Händler) folgt in einem
/// späteren Slice.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Fingerabdruck(NonEmptyString);

impl Fingerabdruck {
    /// Reine Funktion `Buchung` → `Fingerabdruck`. Unfehlbar: die
    /// Empfänger-Identität ist ein bereits validierter `NonEmptyString`,
    /// aus dem der Fingerabdruck direkt gebildet wird — kein Panikpfad.
    pub fn from_buchung(buchung: &Buchung) -> Self {
        Self::from(buchung.empfänger().identity_key().clone())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<NonEmptyString> for Fingerabdruck {
    fn from(text: NonEmptyString) -> Self {
        Self(text)
    }
}
