//! Der **toml-Loader der Fenster-Config** ([Issue 0054](../../../../docs/issues/0054-config-loader-toml-fenster-config.md),
//! [ADR-0013](../../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).
//!
//! `io` liest die Config aus einer Datei und reicht die Paging-Seitengröße [`FensterConfig::window_days`]
//! als **Wert** in den `fenstern`-Slice — genau wie den `seed` bei `obfuskierung`
//! ([`SeedConfig`](super::SeedConfig); die Domäne liest keine Datei). Die **Tag-Stamp-Achse**
//! (Buchungstag, ADR-0013 Entscheidung C) ist eine **fixierte** Determinismus-Entscheidung, kein
//! Config-Knopf: sie kommt als sortierbare `i64`-Werte über den Source-Seam herein, und
//! `window_days` ist die Seitengröße **entlang** dieser Achse. Darum trägt die Config genau **einen**
//! Wert.
//!
//! **Fehlbar + redigiert:** eine fehlende oder ungültige Config schlägt über den geteilten
//! [`ConfigError`](super::ConfigError) sauber fehl — **kein Panic**, und der Fehlertext zitiert
//! **nie** einen Rohwert (weder Dateiinhalt noch Pfad noch den gelesenen Wert), nur die statische
//! Ursache.

use std::num::NonZeroU32;
use std::path::Path;

use super::{ConfigError, ConfigKind};

/// Die geladene Fenster-Config als reiner Wert-Typ: die Paging-Seitengröße des `fenstern`-Pagers.
/// `window_days` ist als [`NonZeroU32`] getypt — eine Seitengröße 0 ist **unkonstruierbar**, nicht
/// „geprüft" (ADR-0005): der Loader lehnt sie am Rand ab, danach trägt der Typ den Beweis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FensterConfig {
    window_days: NonZeroU32,
}

impl FensterConfig {
    /// Die Paging-Seitengröße, die `io` dem [`consumer`](crate::fenstern::consumer) als Wert
    /// hereinreicht.
    pub fn window_days(&self) -> NonZeroU32 {
        self.window_days
    }

    /// Lädt die Config aus einer Datei. Ein unlesbarer/fehlender Pfad ist ein **redigierter**
    /// [`ConfigError::Unreadable`](super::ConfigError::Unreadable) — der Pfad selbst bleibt draußen.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let raw = super::read_config_file(path, ConfigKind::Fenster)?;
        Self::from_toml_str(&raw)
    }

    /// Parst die Config aus einem toml-Text. Öffentlich, damit der Test sie gegen eine eingebettete
    /// Fixture prüfen kann, ohne über das Dateisystem zu gehen.
    pub fn from_toml_str(raw: &str) -> Result<Self, ConfigError> {
        let table = super::parse_toml_table(raw, ConfigKind::Fenster)?;

        let window_days = table
            .get("window_days")
            .ok_or(ConfigError::WindowDaysMissing)?
            .as_integer()
            .and_then(|value| u32::try_from(value).ok())
            .and_then(NonZeroU32::new)
            .ok_or(ConfigError::WindowDaysInvalid)?;

        Ok(Self { window_days })
    }
}
