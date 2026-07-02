use csv_camt_v8_domain::BuchungsartEntity;

/// Das UBQL-DTO für die maschinelle Art einer Buchung am Slice-Rand. Reiner
/// Carrier: das Deuten des rohen Buchungstexts liegt in der Domäne
/// ([`BuchungsartEntity::aus_buchungstext`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buchungsart {
    Lastschrift,
    Dauerauftrag,
    Überweisung,
    Entgelt,
    Sonstige,
}

impl Buchungsart {
    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Projektion).
    pub(crate) fn from_entity(entity: BuchungsartEntity) -> Self {
        match entity {
            BuchungsartEntity::Lastschrift => Self::Lastschrift,
            BuchungsartEntity::Dauerauftrag => Self::Dauerauftrag,
            BuchungsartEntity::Überweisung => Self::Überweisung,
            BuchungsartEntity::Entgelt => Self::Entgelt,
            BuchungsartEntity::Sonstige => Self::Sonstige,
        }
    }
}
