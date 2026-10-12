//! Der **toml-Loader der Spar-Schwelle**
//! ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)).
//!
//! Ein Spar-Kandidat ist **jeder Kostentreiber, dessen Jahresbetrag über einem Config-Wert liegt**
//! (Entscheidung V3 des Abschlussplans): das Programm schlägt vor, der Nutzer entscheidet. Diese
//! Datei trägt genau diesen einen Wert — `jahresbetrag_ab_cent`, der Jahresbetrag in Cent (der
//! kleinsten Einheit der Kontowährung), ab dem ein Treiber vorgeschlagen wird. `io` reicht ihn als
//! **Wert** in den `auswerten`-Slice, wie die Fenstergröße in `fenstern` (die Domäne liest keine
//! Datei).
//!
//! **Eine eigene Datei** statt eines weiteren Schlüssels in der Fenster-Config: die Seitengröße des
//! Pagers ist Mechanik, die Schwelle eine Vorliebe des Nutzers — sie ändern sich aus verschiedenen
//! Gründen. Vertraulich ist sie nicht, darum ist die Datei auch nicht gitignored; die Vorlage liegt
//! als `sparen.example.toml` im Repository-Wurzelverzeichnis.
//!
//! **Fehlbar + redigiert:** eine fehlende oder ungültige Config schlägt über den geteilten
//! [`ConfigError`](super::ConfigError) fehl — kein Panic, kein zitierter Rohwert.

use std::num::NonZeroU64;
use std::path::Path;

use auswerten_adapter::SavingsThresholdConfig;

use super::{ConfigError, ConfigKind};

/// Die geladene Spar-Schwelle als reiner Wert-Typ. Als [`NonZeroU64`] getypt: eine Schwelle von 0
/// machte **jeden** Treiber zum Kandidaten — keine Auswahl mehr, sondern das ganze Ranking noch
/// einmal. Der Loader lehnt sie am Rand ab, danach trägt der Typ den Beweis (ADR-0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparenConfig {
    annual_minor_units: NonZeroU64,
}

impl SparenConfig {
    /// Die Schwelle als Slice-DTO, wie `io` sie der fenster-übergreifenden Reichweite hereinreicht.
    pub fn savings_threshold(&self) -> SavingsThresholdConfig {
        SavingsThresholdConfig {
            annual_minor_units: self.annual_minor_units.get(),
        }
    }

    /// Lädt die Config aus einer Datei. Ein unlesbarer/fehlender Pfad ist ein **redigierter**
    /// [`ConfigError::Unreadable`](super::ConfigError::Unreadable) — der Pfad selbst bleibt draußen.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let raw = super::read_config_file(path, ConfigKind::Sparen)?;
        Self::from_toml_str(&raw)
    }

    /// Parst die Config aus einem toml-Text. Öffentlich, damit der Test sie ohne Dateisystem prüfen
    /// kann.
    pub fn from_toml_str(raw: &str) -> Result<Self, ConfigError> {
        let table = super::parse_toml_table(raw, ConfigKind::Sparen)?;

        let annual_minor_units = table
            .get("jahresbetrag_ab_cent")
            .ok_or(ConfigError::SavingsThresholdMissing)?
            .as_integer()
            .and_then(|value| u64::try_from(value).ok())
            .and_then(NonZeroU64::new)
            .ok_or(ConfigError::SavingsThresholdInvalid)?;

        Ok(Self { annual_minor_units })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn die_schwelle_wird_als_jahresbetrag_in_cent_gelesen() {
        let config = SparenConfig::from_toml_str("jahresbetrag_ab_cent = 12000")
            .expect("gültige Spar-Config");

        assert_eq!(
            config.savings_threshold(),
            SavingsThresholdConfig {
                annual_minor_units: 12_000
            }
        );
    }

    #[test]
    fn die_lauf_fixture_ist_eine_gueltige_spar_config() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/lauf/sparen.toml");

        assert!(SparenConfig::load(&path).is_ok(), "die Lauf-Fixture lädt");
    }

    #[test]
    fn eine_fehlende_schwelle_wird_benannt() {
        assert_eq!(
            SparenConfig::from_toml_str("# leer"),
            Err(ConfigError::SavingsThresholdMissing)
        );
    }

    #[test]
    fn null_negativ_oder_kein_integer_ist_keine_schwelle() {
        for raw in [
            "jahresbetrag_ab_cent = 0",
            "jahresbetrag_ab_cent = -5",
            "jahresbetrag_ab_cent = 120.5",
            "jahresbetrag_ab_cent = \"120\"",
        ] {
            assert_eq!(
                SparenConfig::from_toml_str(raw),
                Err(ConfigError::SavingsThresholdInvalid),
                "{raw}"
            );
        }
    }

    #[test]
    fn kaputtes_toml_und_fehlende_datei_bleiben_redigiert() {
        let malformed = SparenConfig::from_toml_str("jahresbetrag_ab_cent = = 4711")
            .expect_err("kein gültiges TOML");
        let unreadable = SparenConfig::load(Path::new("/gibt/es/nicht/sparen.toml"))
            .expect_err("fehlende Datei");

        assert_eq!(malformed, ConfigError::Malformed(ConfigKind::Sparen));
        assert_eq!(unreadable, ConfigError::Unreadable(ConfigKind::Sparen));
        // Weder der Wert noch der Pfad stehen in der Meldung.
        assert!(!malformed.to_string().contains("4711"));
        assert!(!unreadable.to_string().contains("/gibt/es/nicht"));
    }
}
