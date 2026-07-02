use csv_camt_v8_domain::ObfuskierterTextEntity;

/// Das UBQL-DTO für obfuskierten Freitext (Verwendungszweck) am Slice-Rand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObfuskierterText(ObfuskierterTextEntity);

impl ObfuskierterText {
    /// Baut das DTO aus rohem Text — `None`, wenn nach dem Trimmen leer.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        ObfuskierterTextEntity::new(value).map(Self)
    }

    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Projektion).
    pub(crate) fn from_entity(entity: ObfuskierterTextEntity) -> Self {
        Self(entity)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
