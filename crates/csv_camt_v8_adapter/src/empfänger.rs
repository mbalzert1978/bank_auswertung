use csv_camt_v8_domain::{EmpfängerEntity, GläubigerIdEntity, PseudonymEntity};

/// Das UBQL-DTO für die SEPA-Gläubiger-ID am Slice-Rand — öffentlicher
/// Firmen-Anker (kein struktureller Identifikator einer Person).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GläubigerId(GläubigerIdEntity);

impl GläubigerId {
    /// Baut das DTO aus rohem Text — `None`, wenn nach dem Trimmen leer.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        GläubigerIdEntity::new(value).map(Self)
    }

    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Projektion).
    pub(crate) fn from_entity(entity: GläubigerIdEntity) -> Self {
        Self(entity)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Das UBQL-DTO für das Personen-Pseudonym (`PERSON_x`) am Slice-Rand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pseudonym(PseudonymEntity);

impl Pseudonym {
    /// Baut das DTO aus rohem Text — `None`, wenn nach dem Trimmen leer.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        PseudonymEntity::new(value).map(Self)
    }

    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Projektion).
    pub(crate) fn from_entity(entity: PseudonymEntity) -> Self {
        Self(entity)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Das UBQL-DTO für die Gegenpartei einer Buchung — `Firma` über die Gläubiger-ID,
/// `Person` über ein stabiles Pseudonym; ein Klarname ist in keinem Fall enthalten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Empfänger {
    Firma(GläubigerId),
    Person(Pseudonym),
}

impl Empfänger {
    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Projektion).
    pub(crate) fn from_entity(entity: &EmpfängerEntity) -> Self {
        match entity {
            EmpfängerEntity::Firma(id) => Self::Firma(GläubigerId::from_entity(id.clone())),
            EmpfängerEntity::Person(pseudonym) => {
                Self::Person(Pseudonym::from_entity(pseudonym.clone()))
            }
        }
    }
}
