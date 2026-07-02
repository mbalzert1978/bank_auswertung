use csv_camt_v8_domain::KontoIdEntity;

/// Das UBQL-DTO für die obfuskierte Konto-Kennung (`KONTO_x`) am Slice-Rand — nie
/// die echte IBAN. Unbedenklich: kein struktureller Identifikator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KontoId(KontoIdEntity);

impl KontoId {
    /// Baut das DTO aus rohem Text — `None`, wenn nach dem Trimmen leer.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        KontoIdEntity::new(value).map(Self)
    }

    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Projektion).
    pub(crate) fn from_entity(entity: KontoIdEntity) -> Self {
        Self(entity)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
