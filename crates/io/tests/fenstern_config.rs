//! io-Integrationstest ([Issue 0054](../../../docs/issues/0054-config-loader-toml-fenster-config.md)):
//! der **toml-Loader der Fenster-Config**. `io` liest die Config aus einer Datei und reicht die
//! Paging-Seitengröße `window_days` als **Wert** in den `fenstern`-Slice — die Tag-Stamp-Achse
//! (der Buchungstag, ADR-0013 Entscheidung C) ist **fixiert** und kommt als `i64`-Werte über den
//! Source-Seam, nicht aus der Config; die Config parametrisiert nur die Seitengröße **entlang**
//! dieser Achse. Der Slice liest weiterhin **keine** Datei und bindet keinen Kalender.
//!
//! Getestet wird der Loader an seiner öffentlichen Oberfläche: eine gültige Fixture → der erwartete
//! Wert; eine fehlende **und** eine ungültige Config → je ein sauberer, **redigierter** Fehlschlag
//! (kein Panic, kein Rohwert im Fehlertext).

use std::path::Path;

use io::fenstern::{ConfigError, FensterConfig};

/// Die gültige Fixture-Config, gegen die der Loader den erwarteten Wert liefern muss.
const VALID: &str = include_str!("../fixtures/fenstern_config/valid.toml");

// --- Happy Path: gültige Config → erwartete Seitengröße ----------------------------------

#[test]
fn eine_gueltige_config_liefert_die_erwartete_seitengroesse() {
    let config = FensterConfig::from_toml_str(VALID).expect("gültige Config lädt");

    assert_eq!(config.window_days().get(), 3);
}

// --- Fehlende Config: sauberer, redigierter Fehlschlag -----------------------------------

#[test]
fn eine_fehlende_config_schlaegt_redigiert_fehl() {
    // Ein Pfad, unter dem keine Datei liegt — der Loader gibt ein `Result`, nie einen Panic.
    let fehler = FensterConfig::load(Path::new(
        "crates/io/fixtures/fenstern_config/existiert-nicht.toml",
    ))
    .expect_err("eine fehlende Datei muss fehlschlagen");

    // Redigiert: der Fehler nennt die statische Ursache, nie den Pfad (kein Rohwert).
    assert_eq!(fehler, ConfigError::Unreadable);
    assert!(!fehler.to_string().contains("existiert-nicht"));
}

// --- Ungültige Config: sauberer, redigierter Fehlschlag ----------------------------------

#[test]
fn eine_ungueltige_config_schlaegt_redigiert_fehl() {
    // `window_days` ist kein Integer → ungültig; der gelesene Rohwert darf im Fehler nicht auftauchen.
    let fehler = FensterConfig::from_toml_str("window_days = \"drei\"")
        .expect_err("ein ungültiger Wert muss fehlschlagen");

    assert_eq!(fehler, ConfigError::WindowDaysInvalid);
    assert!(!fehler.to_string().contains("drei"));
}
