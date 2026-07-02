use csv_camt_v8_domain::GegenkontoEntity;

use crate::rolle::Rolle;

/// Das UBQL-DTO für die Gegenseite einer Buchung am Slice-Rand: eigenes Konto
/// (`Eigen`, mit Rolle — dann eine `Umbuchung`) oder fremdes (`Fremd`). „Eigen ohne
/// Rolle" ist unkonstruierbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gegenkonto {
    Eigen(Rolle),
    Fremd,
}

impl Gegenkonto {
    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Projektion).
    pub(crate) fn from_entity(entity: GegenkontoEntity) -> Self {
        match entity {
            GegenkontoEntity::Eigen(rolle) => Self::Eigen(Rolle::from_entity(rolle)),
            GegenkontoEntity::Fremd => Self::Fremd,
        }
    }
}
