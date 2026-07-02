use std::str::FromStr;

use eigene_konten_domain::RolleEntity;

/// Das UBQL-DTO für die Konto-Rolle am Slice-Rand (schlichter Glossar-Name, kein
/// `Dto`-Suffix). Der Config-Text liefert sie als Zeichenkette; `from_str`
/// übersetzt in dieses DTO, die Bridges übersetzen weiter in die Domänen-`Rolle`
/// ([`RolleEntity`]).
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
    /// Übersetzt das DTO in den Domänen-Wert — crate-intern für die Bridges.
    pub(crate) fn entity(self) -> RolleEntity {
        match self {
            Self::Giro => RolleEntity::Giro,
            Self::Haushalt => RolleEntity::Haushalt,
            Self::Spar => RolleEntity::Spar,
            Self::Vorlauf => RolleEntity::Vorlauf,
        }
    }

    /// Übersetzt einen Domänen-Wert zurück in das DTO — crate-intern.
    pub(crate) fn from_entity(entity: RolleEntity) -> Self {
        match entity {
            RolleEntity::Giro => Self::Giro,
            RolleEntity::Haushalt => Self::Haushalt,
            RolleEntity::Spar => Self::Spar,
            RolleEntity::Vorlauf => Self::Vorlauf,
        }
    }
}

impl FromStr for Rolle {
    type Err = RolleError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "Giro" => Ok(Self::Giro),
            "Haushalt" => Ok(Self::Haushalt),
            "Spar" => Ok(Self::Spar),
            "Vorlauf" => Ok(Self::Vorlauf),
            other => Err(RolleError(other.to_string())),
        }
    }
}

/// Eine Konto-Rolle, die es nicht gibt.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unbekannte Konto-Rolle „{0}“ (erlaubt: Giro, Haushalt, Spar, Vorlauf)")]
pub struct RolleError(String);
