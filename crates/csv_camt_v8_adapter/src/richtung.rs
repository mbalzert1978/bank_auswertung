use csv_camt_v8_domain::RichtungEntity;

/// Das UBQL-DTO für die Richtung einer Buchung am Slice-Rand. Trägt das
/// Vorzeichen; ein negativer `Eingang` ist unkonstruierbar (das Vorzeichen wird
/// beim Parsen zur Richtung, der Betrag bleibt vorzeichenlos).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Richtung {
    Eingang,
    Ausgang,
}

impl Richtung {
    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Projektion).
    pub(crate) fn from_entity(entity: RichtungEntity) -> Self {
        match entity {
            RichtungEntity::Eingang => Self::Eingang,
            RichtungEntity::Ausgang => Self::Ausgang,
        }
    }
}
