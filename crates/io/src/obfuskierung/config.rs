//! Der **toml-Loader des Obfuskierungs-Seeds** ([Issue 0059](../../../../docs/issues/0059-config-loader-toml-seed.md),
//! [ADR-0010](../../../../docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)).
//!
//! `io` liest den Seed aus einer Datei und reicht ihn als rohen `u64`-**Wert** in den
//! `obfuskierung`-Slice ([`consumer`](crate::obfuskierung::consumer)) — genau das Muster der
//! Fenster-Config ([`FensterConfig`](crate::fenstern::FensterConfig)), nur für den Seed: der Slice
//! liest **keine** Datei, `io` liest die Config und übergibt den fertigen Wert. Damit steht der
//! letzte hereingereichte Wert des `obfuskierung`-Laufs auf einer echten Quelle: gleicher Seed →
//! gleiche `PERSON_x`/`KONTO_x` über Importe hinweg (ADR-0010). Der Seed bleibt dabei **lokal** — er
//! erreicht nie ein Egress-DTO.
//!
//! **Fehlbar + redigiert:** eine fehlende oder ungültige Config schlägt über [`ConfigError`] sauber
//! fehl — **kein Panic**, und der Fehlertext zitiert **nie** einen Rohwert (weder Dateiinhalt noch
//! Pfad noch den gelesenen Seed), nur die statische Ursache. Ein geleakter Seed ist so schädlich wie
//! ein geleakter Klartext, darum bleibt er strikt aus dem Fehler-Chain.

use std::path::Path;

/// Der geladene Obfuskierungs-Seed als reiner Wert-Typ. Trägt genau **einen** Wert, den rohen
/// `u64`-Seed, den `io` dem `obfuskierung`-Consumer als Wert hereinreicht. Der Wert selbst wird
/// bewusst **nicht** über [`Debug`](std::fmt::Debug)/[`Display`](std::fmt::Display) o. Ä. ausgegeben
/// — er bleibt lokal (ADR-0010).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedConfig {
    seed: u64,
}

impl SeedConfig {
    /// Der Seed, den `io` dem [`consumer`](crate::obfuskierung::consumer) als Wert hereinreicht. Ein
    /// Seed von `0` ist ein **gültiger** Seed (anders als eine Fenster-Seitengröße von 0), darum
    /// trägt der Wert keine `NonZero`-Schranke — nur der Rand lehnt ab, was gar keine
    /// nicht-negative Ganzzahl ist.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Lädt den Seed aus einer Datei. Ein unlesbarer/fehlender Pfad ist ein **redigierter**
    /// [`ConfigError::Unreadable`] — der Pfad selbst bleibt draußen.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        // Die io-Fehlermeldung wird bewusst **verworfen** (nicht als `source` gehängt): sie könnte den
        // Pfad tragen, und redigiert heißt kein Rohwert im Fehler-Chain.
        let raw = std::fs::read_to_string(path).map_err(|_err| ConfigError::Unreadable)?;
        Self::from_toml_str(&raw)
    }

    /// Parst den Seed aus einem toml-Text. Öffentlich, damit der Test ihn gegen eine eingebettete
    /// Fixture prüfen kann, ohne über das Dateisystem zu gehen.
    pub fn from_toml_str(raw: &str) -> Result<Self, ConfigError> {
        // Der toml-Parsefehler wird verworfen — er kann Zeilen-/Feldinhalt (bis hin zum Seed) zitieren;
        // redigiert heißt, nur die statische Ursache reist weiter.
        let table: toml::Table = raw.parse().map_err(|_err| ConfigError::Malformed)?;

        let seed = table
            .get("seed")
            .ok_or(ConfigError::SeedMissing)?
            .as_integer()
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(ConfigError::SeedInvalid)?;

        Ok(Self { seed })
    }
}

/// Warum das Laden des Seeds misslang — **redigiert**: jede Variante trägt eine **statische** Ursache
/// und **nie** einen Rohwert (Dateiinhalt, Pfad, gelesener Seed). So kann der Fehler nirgends zum
/// Leck-Pfad werden, und der Aufrufer bleibt panikfrei.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// Die Config-Datei ließ sich nicht lesen (fehlt oder ist nicht zugänglich).
    Unreadable,
    /// Der Inhalt ist kein gültiges TOML.
    Malformed,
    /// Der Schlüssel `seed` fehlt.
    SeedMissing,
    /// `seed` ist keine nicht-negative Ganzzahl (kein Integer, negativ oder außerhalb von `u64`).
    SeedInvalid,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self {
            Self::Unreadable => "Seed-Konfiguration ist nicht lesbar",
            Self::Malformed => "Seed-Konfiguration ist kein gültiges TOML",
            Self::SeedMissing => "Seed-Konfiguration: seed fehlt",
            Self::SeedInvalid => "Seed-Konfiguration: seed muss eine nicht-negative Ganzzahl sein",
        };
        f.write_str(reason)
    }
}

impl std::error::Error for ConfigError {}
