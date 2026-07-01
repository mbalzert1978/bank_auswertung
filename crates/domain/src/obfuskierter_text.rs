use crate::NonEmptyString;

/// Freitext (Empfänger/Verwendungszweck) nach Obfuskierung. Minimale
/// Erstversion: trimmt nur (via `NonEmptyString`) und lehnt Leeres ab —
/// konsistent mit den übrigen Value Objects. Das eigentliche Scrubbing
/// eingebetteter Identifikatoren (IBAN-/BIC-Muster, bekannte Personen-Token)
/// vertieft ein späterer Slice (ADR-0003); dieser Typ erzwingt schon jetzt
/// den einen Engpass, durch den aller Freitext muss.
///
/// „Kein Verwendungszweck" ist keine Sache dieses Typs, sondern des Feldes:
/// die Abwesenheit wird ab Schritt 2 als `Option<ObfuskierterText>` am
/// `Buchung`-Feld modelliert — nicht als leerer `ObfuskierterText`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ObfuskierterText(NonEmptyString);

impl ObfuskierterText {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<NonEmptyString> for ObfuskierterText {
    fn from(text: NonEmptyString) -> Self {
        Self(text)
    }
}
