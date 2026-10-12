//! Der **toml-Loader der lokalen Namensliste** `personen.toml`
//! ([Issue 0010](../../../../docs/issues/0010-scrubbing-regeln-fuer-obfuskiertertext.md)): die Namen,
//! die der Freitext-Engpass des `obfuskierung`-Slice im Verwendungszweck ersetzt, ohne dass `io` sie
//! selbst kennt: Namen, die nie Empfänger waren, und **Teilnamen** von Empfängern (Vornamen,
//! Kurzformen) — die vollständigen Empfänger-Namen kennt `io` aus dem einlesen-Bestand, Teilnamen
//! davon schwärzt der Engpass nicht von selbst.
//!
//! Format wie die Vorlage `personen.example.toml` im Repo-Wurzelverzeichnis:
//!
//! ```toml
//! namen = ["Erika Mustermann", "Max Mustermann"]
//! ```
//!
//! `namen = []` ist gültig — die bewusste Aussage „keine weiteren Namen". Der Schlüssel selbst ist
//! Pflicht, und die Datei ist für den Lauf Pflicht: eine vergessene Liste soll laut scheitern, statt
//! still weniger zu scrubben (fail-safe).
//!
//! **Keine Klartext↔Token-Zuordnung** ([ADR-0010](../../../../docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)):
//! die Liste trägt nur Namen. Sie bleibt lokal (die echte `personen.toml` ist gitignored).
//!
//! **Fehlbar + redigiert:** jeder Fehler nennt die statische Ursache, **nie** einen Namen, den Pfad
//! oder den Dateiinhalt; die `Debug`-Ausgabe nennt nur die Anzahl.

use std::path::Path;

use super::{ConfigError, ConfigKind};

/// Die geladene Namensliste als reiner Wert-Typ. Die Namen sind Klartext — darum **kein**
/// `#[derive(Debug)]`, sondern eine Debug-Ausgabe, die nur zählt.
#[derive(Clone, PartialEq, Eq)]
pub struct PersonenConfig {
    names: Vec<String>,
}

impl std::fmt::Debug for PersonenConfig {
    /// Redigiert: nur die Anzahl der Namen, nie ein Name.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersonenConfig")
            .field("names", &self.names.len())
            .finish()
    }
}

impl PersonenConfig {
    /// Die Namen in Datei-Reihenfolge, getrimmt.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Gibt die Namen heraus — für den Namens-Seam, der sie mit den Empfänger-Namen zusammenführt.
    pub fn into_names(self) -> Vec<String> {
        self.names
    }

    /// Lädt die Liste aus einer Datei. Ein unlesbarer/fehlender Pfad ist ein **redigierter**
    /// [`ConfigError::Unreadable`] — der Pfad selbst bleibt draußen.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let raw = super::read_config_file(path, ConfigKind::Personen)?;
        Self::from_toml_str(&raw)
    }

    /// Parst die Liste aus einem toml-Text. Öffentlich, damit der Test sie gegen eine eingebettete
    /// Fixture prüfen kann, ohne über das Dateisystem zu gehen.
    pub fn from_toml_str(raw: &str) -> Result<Self, ConfigError> {
        let table = super::parse_toml_table(raw, ConfigKind::Personen)?;

        let names = table
            .get("namen")
            .ok_or(ConfigError::PersonNamesMissing)?
            .as_array()
            .ok_or(ConfigError::PersonNamesInvalid)?
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .map(str::trim)
                    .filter(|name| has_name_part(name))
                    .map(str::to_string)
                    .ok_or(ConfigError::PersonNamesInvalid)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { names })
    }
}

/// Ob ein Eintrag mindestens einen Namensteil trägt — dieselbe Trennung (Leerraum, Komma), mit der
/// der Freitext-Engpass einen Namen in seine Teile zerlegt. Ein Eintrag wie `","` hätte keinen und
/// wäre ein stiller Nicht-Name; er scheitert darum wie ein leerer.
fn has_name_part(name: &str) -> bool {
    name.split(|c: char| c.is_whitespace() || c == ',')
        .any(|part| !part.is_empty())
}
