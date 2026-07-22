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
    // Ein Pfad, unter dem keine Datei liegt — der Loader gibt ein `Result`, nie einen Panic. Der Pfad
    // wird crate-relativ über `CARGO_MANIFEST_DIR` gebaut (wie der `architecture`-Test), damit er nicht
    // vom Working Directory des Test-Runners abhängt.
    let fehlender_pfad =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/fenstern_config/existiert-nicht.toml");
    let fehler =
        FensterConfig::load(&fehlender_pfad).expect_err("eine fehlende Datei muss fehlschlagen");

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

// --- Fehlender Schlüssel: eigener, redigierter Fehlschlag --------------------------------

#[test]
fn eine_config_ohne_window_days_schlaegt_redigiert_fehl() {
    // Gültiges (leeres) TOML, aber der Pflichtschlüssel fehlt — abgegrenzt von „kein TOML".
    let fehler =
        FensterConfig::from_toml_str("").expect_err("ein fehlender Schlüssel muss fehlschlagen");

    assert_eq!(fehler, ConfigError::WindowDaysMissing);
}

// --- Kaputtes TOML: eigener, redigierter Fehlschlag (kein Rohwert) -----------------------

#[test]
fn eine_kaputte_config_schlaegt_redigiert_fehl() {
    // Syntaktisch kaputtes TOML mit einem wiedererkennbaren Token: der Parsefehler darf es nicht in
    // den redigierten Fehler durchreichen (der toml-Fehler zitiert sonst gern die kaputte Zeile).
    let fehler =
        FensterConfig::from_toml_str("geheimnis").expect_err("kaputtes TOML muss fehlschlagen");

    assert_eq!(fehler, ConfigError::Malformed);
    assert!(!fehler.to_string().contains("geheimnis"));
}

// --- Randfall window_days = 0: unkonstruierbare Seitengröße (ADR-0005) -------------------

#[test]
fn window_days_null_schlaegt_redigiert_fehl() {
    // Eine Seitengröße 0 ist am Rand abzulehnen (`NonZeroU32`); danach trägt der Typ den Beweis.
    let fehler = FensterConfig::from_toml_str("window_days = 0")
        .expect_err("window_days = 0 muss fehlschlagen");

    assert_eq!(fehler, ConfigError::WindowDaysInvalid);
}
