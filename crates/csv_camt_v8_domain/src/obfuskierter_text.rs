use crate::non_empty_string::NonEmptyString;

/// Freitext (Verwendungszweck) nach der Obfuskierung — der Domänen-Wert, den die
/// Buchung trägt. Die CSV-Domäne erhält den Text bereits durch den
/// Obfuskierungs-Engpass gereicht (Scrubbing eingebetteter Identifikatoren ist
/// dort beheimatet und vertagt, ADR-0003) und hält ihn nur; garantiert
/// nicht-leer.
///
/// „Kein Verwendungszweck" ist keine Sache dieses Typs, sondern des Feldes: die
/// Abwesenheit wird als `Option<ObfuskierterTextEntity>` am Aggregat modelliert,
/// nicht als leerer Text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObfuskierterTextEntity(NonEmptyString);

impl ObfuskierterTextEntity {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
