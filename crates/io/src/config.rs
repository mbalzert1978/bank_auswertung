//! Die **App-Konfiguration** der Infrastruktur (`io`): die von außen gesetzten Werte, aus denen `io`
//! seine realen Adapter aufbaut. Heute trägt sie **einen** Wert — den Pfad der SQLite-Outbox-Datei
//! ([`AppConfig::database_path`]) —, wächst aber um die weiteren io-seitigen Einstellungen
//! (Seed, Fenstergröße, Eigene-Konten-Pfad …), sobald deren Loader landen.
//!
//! **Woher die Werte kommen:** heute konstruiert der Aufrufer die `AppConfig` direkt (Test/ephemer);
//! der reale **TOML-Loader** ([Issue 0054](../../../../docs/issues/0054-config-loader-toml-fenster-config.md))
//! liest sie später aus einer Konfigurationsdatei. So bleibt die eine ripple-schwere Entscheidung —
//! *wohin* die Outbox persistiert — **konfigurierbar** statt im Code verdrahtet.

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
