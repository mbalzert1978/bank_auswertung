//! Die **App-Konfiguration** der Infrastruktur (`io`): die von außen gesetzten Werte, aus denen `io`
//! seine realen Adapter aufbaut, **plus die zentral hier lebenden Config-Loader**. Die
//! [`AppConfig`] trägt heute **einen** Wert — den Pfad der SQLite-Outbox-Datei
//! ([`AppConfig::database_path`]) —, wächst aber um die weiteren io-seitigen Einstellungen, sobald
//! deren Loader landen.
//!
//! **Woher die Werte kommen:** heute konstruiert der Aufrufer die `AppConfig` direkt (Test/ephemer);
//! ein realer **TOML-Loader** liest sie später aus einer Konfigurationsdatei. So bleibt die eine
//! ripple-schwere Entscheidung — *wohin* die Outbox persistiert — **konfigurierbar** statt im Code
//! verdrahtet.
//!
//! **Die Config-Loader leben zentral hier**, nicht mehr an den einzelnen Slices: [`FensterConfig`]
//! liest die Paging-Seitengröße des `fenstern`-Slice aus toml
//! ([Issue 0054](../../../docs/issues/0054-config-loader-toml-fenster-config.md)) und
//! [`SeedConfig`] den Obfuskierungs-Seed des `obfuskierung`-Slice
//! ([Issue 0059](../../../docs/issues/0059-config-loader-toml-seed.md)); die weiteren io-seitigen
//! Einstellungen (Eigene-Konten-Pfad …) folgen demselben Muster. **Beide Loader teilen sich einen
//! einzigen** [`ConfigError`] — die redigierte, statische Fehler-Ursache jedes fehlgeschlagenen
//! Ladens; `io` reicht den geladenen Wert dann als rohen Wert in den jeweiligen Consumer (die
//! Domäne liest **keine** Datei). Der zugehörige [`ConfigKind`] ist **bewusst Teil der öffentlichen
//! API**: er ist der typsichere, redigierte Loader-Diskriminant der beiden geteilten Fehlerformen
//! ([`Unreadable`](ConfigError::Unreadable)/[`Malformed`](ConfigError::Malformed)) — ein Summentyp
//! statt eines stringly-typed Labels (ADR-0005) — und muss öffentlich sein, sobald der öffentliche
//! [`ConfigError`] ihn trägt.

mod fenster;
mod seed;

pub use fenster::FensterConfig;
pub use seed::SeedConfig;

use std::path::{Path, PathBuf};

/// Die von außen gesetzte Konfiguration, aus der `io` seine Adapter aufbaut. Ein reiner Wert-Typ —
/// er liest **nichts** selbst; wer ihn füllt (Test heute, TOML-Loader später), ist seine Sache.
#[derive(Debug, Clone)]
pub struct AppConfig {
    database_path: PathBuf,
}

impl AppConfig {
    /// Baut die Konfiguration aus dem Pfad der SQLite-Outbox-Datei.
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
        }
    }

    /// Der Pfad, unter dem das transaktionale Rückgrat persistiert
    /// ([`SqliteOutbox::open`](crate::sqlite::SqliteOutbox::open)).
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }
}

/// Welcher Loader den Fehler erlitt — der **statische** Kontext der geteilten
/// [`ConfigError`]-Varianten [`Unreadable`](ConfigError::Unreadable) und
/// [`Malformed`](ConfigError::Malformed). Rein statisch und redigiert: die interne `label`-Methode
/// gibt nur eine feste Bezeichnung zurück, **nie** einen Rohwert (Pfad, Dateiinhalt, Seed/Wert).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigKind {
    /// Der Obfuskierungs-Seed-Loader ([`SeedConfig`]).
    Seed,
    /// Der Fenster-Config-Loader ([`FensterConfig`]).
    Fenster,
}

impl ConfigKind {
    /// Die **statische**, redigierte Bezeichnung des Loaders für die Fehlermeldung — nie ein Rohwert.
    fn label(self) -> &'static str {
        match self {
            Self::Seed => "Seed-Konfiguration",
            Self::Fenster => "Fenster-Konfiguration",
        }
    }
}

/// Warum das Laden einer Config misslang — **der eine, geteilte** Fehler beider Loader
/// ([`SeedConfig`] und [`FensterConfig`]). **Redigiert**: jede Variante trägt eine **statische**
/// Ursache und **nie** einen Rohwert (Dateiinhalt, Pfad, gelesener Seed/Wert). Die beiden geteilten
/// Fehlerformen ([`Unreadable`](Self::Unreadable)/[`Malformed`](Self::Malformed)) tragen ihren
/// Loader-Kontext als statischen [`ConfigKind`] mit, damit die Meldung loader-spezifisch bleibt. So
/// kann der Fehler nirgends zum Leck-Pfad werden, und der Aufrufer bleibt panikfrei.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// Die Config-Datei ließ sich nicht lesen (fehlt oder ist nicht zugänglich).
    Unreadable(ConfigKind),
    /// Der Inhalt ist kein gültiges TOML.
    Malformed(ConfigKind),
    /// Der Schlüssel `seed` fehlt.
    SeedMissing,
    /// `seed` ist keine nicht-negative Ganzzahl (kein Integer, negativ oder außerhalb von `u64`).
    SeedInvalid,
    /// Der Schlüssel `window_days` fehlt.
    WindowDaysMissing,
    /// `window_days` ist keine positive Ganzzahl (kein Integer, negativ, zu groß oder null).
    WindowDaysInvalid,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(kind) => write!(f, "{} ist nicht lesbar", kind.label()),
            Self::Malformed(kind) => write!(f, "{} ist kein gültiges TOML", kind.label()),
            Self::SeedMissing => f.write_str("Seed-Konfiguration: seed fehlt"),
            Self::SeedInvalid => {
                f.write_str("Seed-Konfiguration: seed muss eine nicht-negative Ganzzahl sein")
            }
            Self::WindowDaysMissing => f.write_str("Fenster-Konfiguration: window_days fehlt"),
            Self::WindowDaysInvalid => {
                f.write_str("Fenster-Konfiguration: window_days muss eine positive Ganzzahl sein")
            }
        }
    }
}

impl std::error::Error for ConfigError {}
