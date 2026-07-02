use obfuskierung_domain::{KontoIdEntity, ObfuskierterTextEntity, PseudonymEntity};

/// Das UBQL-DTO für die obfuskierte Konto-Kennung (`KONTO_x`) — der **Ausgang**
/// des Slice, in dem er nach außen (auch Slice→Slice) spricht. Unbedenklich: kein
/// struktureller Identifikator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KontoId(KontoIdEntity);

impl KontoId {
    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Bridge).
    pub(crate) fn from_entity(entity: KontoIdEntity) -> Self {
        Self(entity)
    }

    /// Das Token als Text (`KONTO_x`).
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Das UBQL-DTO für das Personen-Pseudonym (`PERSON_x`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pseudonym(PseudonymEntity);

impl Pseudonym {
    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Bridge).
    pub(crate) fn from_entity(entity: PseudonymEntity) -> Self {
        Self(entity)
    }

    /// Das Token als Text (`PERSON_x`).
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Das UBQL-DTO für obfuskierten Freitext (Empfänger/Verwendungszweck).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObfuskierterText(ObfuskierterTextEntity);

impl ObfuskierterText {
    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Bridge).
    pub(crate) fn from_entity(entity: ObfuskierterTextEntity) -> Self {
        Self(entity)
    }

    /// Der obfuskierte Text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
