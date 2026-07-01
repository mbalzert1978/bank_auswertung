use std::collections::HashMap;

use serde::Deserialize;

use crate::config_source::{ConfigSource, ConfigSourceError};
use crate::iban::Iban;
use crate::kontozuordnung::Kontozuordnung;
use crate::rolle::Rolle;

/// Die Eigene-Konten-Config: je eigenem Konto seine IBAN und Rolle.
///
/// Die **eine Quelle** für die Eigen/Fremd-Auflösung beim CSV-Einlesen und die
/// Rollen-Deutung der Analyse — [`lookup`](Self::lookup) beantwortet beide
/// Fragen aus derselben Instanz.
#[derive(Debug, Clone)]
pub struct EigeneKonten {
    konten: HashMap<Iban, Rolle>,
}

impl EigeneKonten {
    /// Lädt die Config über eine [`ConfigSource`] (Port) und parst sie. Ein
    /// Lese-, TOML-, IBAN- oder Rollenfehler oder eine doppelte IBAN führt zum
    /// Abbruch — kein stillschweigendes Überspringen.
    pub fn from_source(source: &impl ConfigSource) -> Result<Self, LoadError> {
        let content = source.read()?;
        Self::from_toml(&content)
    }

    /// Parst die Config aus einem bereits gelesenen TOML-String: der reine,
    /// I/O-freie Kern (Prüfung am öffentlichen Kontrakt).
    pub fn from_toml(content: &str) -> Result<Self, LoadError> {
        let raw: RawConfig = toml::from_str(content)?;
        let konten = raw
            .konto
            .into_iter()
            .try_fold(HashMap::new(), |mut acc, entry| {
                if acc.insert(entry.iban.clone(), entry.rolle).is_some() {
                    Err(LoadError::DuplicateIban(entry.iban))
                } else {
                    Ok(acc)
                }
            })?;
        Ok(Self { konten })
    }

    /// Beantwortet für eine Gegen-IBAN beide Fragen zugleich: eigen (mit Rolle)
    /// oder fremd.
    pub fn lookup(&self, iban: &Iban) -> Kontozuordnung {
        self.konten
            .get(iban)
            .copied()
            .map_or(Kontozuordnung::Fremd, Kontozuordnung::Eigen)
    }
}

/// Zwischenschritt der Deserialisierung: `[[konto]]`-Einträge, bevor Eindeutig-
/// keit geprüft und in die Nachschlage-Map überführt wird. `konto` ist bewusst
/// **nicht** `#[serde(default)]`: eine Config ganz ohne Eintrag ist eine
/// Fehlkonfiguration — die eine Quelle für Eigen/Fremd wäre leer — und wird als
/// fehlendes Feld abgelehnt, statt still „alles fremd" zu bedeuten.
#[derive(Deserialize)]
struct RawConfig {
    konto: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    iban: Iban,
    rolle: Rolle,
}

/// Fehler beim Laden der Eigene-Konten-Config.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// Die Config konnte über die Quelle nicht gelesen werden (z. B. Datei-I/O).
    #[error("Config konnte nicht gelesen werden: {0}")]
    Source(#[from] ConfigSourceError),
    /// Kein gültiges TOML, ungültige IBAN oder unbekannte Rolle — die
    /// Deserialisierung bricht ab, statt einen Eintrag zu überspringen.
    #[error("Config nicht einlesbar: {0}")]
    Toml(#[from] toml::de::Error),
    /// Dieselbe eigene IBAN ist mehrfach eingetragen.
    #[error("Eigene IBAN mehrfach eingetragen: {}", .0.as_str())]
    DuplicateIban(Iban),
}
