use csv_camt_v8_domain::RolleEntity;
use eigene_konten_adapter::Rolle as KontenRolle;

/// Das UBQL-DTO für die Konto-Rolle am Slice-Rand (schlichter Glossar-Name).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rolle {
    /// Laufendes Konto.
    Giro,
    /// Haushaltskonto.
    Haushalt,
    /// Spar-Konto.
    Spar,
    /// Für anstehende Fixkosten reserviert.
    Vorlauf,
}

impl Rolle {
    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Projektion).
    pub(crate) fn from_entity(entity: RolleEntity) -> Self {
        match entity {
            RolleEntity::Giro => Self::Giro,
            RolleEntity::Haushalt => Self::Haushalt,
            RolleEntity::Spar => Self::Spar,
            RolleEntity::Vorlauf => Self::Vorlauf,
        }
    }
}

/// Mappt die **fremde** Rolle des Eigene-Konten-Slice auf die slice-eigene
/// Domänen-`RolleEntity` — die Grenze zwischen den beiden Slices (kein geteilter
/// Typ, ADR-0007). Wird in der Konten-Bridge beim Auflösen von `Eigen` gebraucht.
pub(crate) fn rolle_entity_aus_konten(rolle: KontenRolle) -> RolleEntity {
    match rolle {
        KontenRolle::Giro => RolleEntity::Giro,
        KontenRolle::Haushalt => RolleEntity::Haushalt,
        KontenRolle::Spar => RolleEntity::Spar,
        KontenRolle::Vorlauf => RolleEntity::Vorlauf,
    }
}
