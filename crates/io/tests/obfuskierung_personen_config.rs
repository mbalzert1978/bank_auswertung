//! io-Integrationstest ([Issue 0010](../../../docs/issues/0010-scrubbing-regeln-fuer-obfuskiertertext.md)):
//! der **toml-Loader der lokalen Namensliste** (`personen.toml`). `io` liest die Namen aus einer Datei
//! und reicht sie — zusammen mit den Empfänger-Namen aus dem einlesen-Bestand — als **Wert** in den
//! Freitext-Engpass des `obfuskierung`-Slice. Der Slice liest **keine** Datei.
//!
//! Getestet wird der Loader an seiner öffentlichen Oberfläche: eine gültige Fixture → die erwarteten
//! Namen; jede kaputte Form → ein sauberer, **redigierter** Fehlschlag (kein Panic, kein Name, kein
//! Pfad im Fehlertext). Alle Namen sind erfunden.

use std::path::Path;

use io::config::{ConfigError, ConfigKind, PersonenConfig};

/// Die gültige Fixture-Config, gegen die der Loader die erwarteten Namen liefern muss.
const VALID: &str = include_str!("../fixtures/obfuskierung_personen/valid.toml");

#[test]
fn eine_gueltige_config_liefert_die_getrimmten_namen() {
    let config = PersonenConfig::from_toml_str(VALID).expect("gültige Config lädt");

    assert_eq!(config.names(), ["Lisa Beispiel", "Paul Erfunden"]);
}

#[test]
fn der_dateisystem_pfad_load_liest_die_fixture_erfolgreich() {
    let pfad =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/obfuskierung_personen/valid.toml");
    let config = PersonenConfig::load(&pfad).expect("die gültige Fixture-Datei lädt über load");

    assert_eq!(config.names().len(), 2);
}

#[test]
fn eine_leere_liste_ist_gueltig() {
    // `namen = []` ist die bewusste Aussage „keine weiteren Namen" — kein Fehler.
    let config = PersonenConfig::from_toml_str("namen = []").expect("leere Liste lädt");

    assert!(config.names().is_empty());
}

#[test]
fn die_debug_ausgabe_nennt_keinen_namen() {
    let config = PersonenConfig::from_toml_str(VALID).expect("gültige Config lädt");

    let debug = format!("{config:?}");
    assert!(!debug.contains("Lisa"));
    assert!(!debug.contains("Erfunden"));
}

#[test]
fn eine_fehlende_config_schlaegt_redigiert_fehl() {
    let fehlender_pfad = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/obfuskierung_personen/existiert-nicht.toml");
    let fehler =
        PersonenConfig::load(&fehlender_pfad).expect_err("eine fehlende Datei muss fehlschlagen");

    assert_eq!(fehler, ConfigError::Unreadable(ConfigKind::Personen));
    assert!(!fehler.to_string().contains("existiert-nicht"));
}

#[test]
fn eine_config_ohne_namen_schlaegt_redigiert_fehl() {
    let fehler =
        PersonenConfig::from_toml_str("").expect_err("ein fehlender Schlüssel muss fehlschlagen");

    assert_eq!(fehler, ConfigError::PersonNamesMissing);
}

#[test]
fn ein_nicht_text_eintrag_schlaegt_redigiert_fehl() {
    let fehler = PersonenConfig::from_toml_str("namen = [\"Lisa Beispiel\", 7]")
        .expect_err("ein Eintrag, der kein Text ist, muss fehlschlagen");

    assert_eq!(fehler, ConfigError::PersonNamesInvalid);
    // Redigiert: auch der gültige Nachbar-Eintrag erscheint nicht im Fehler.
    assert!(!fehler.to_string().contains("Lisa"));
}

#[test]
fn ein_leerer_eintrag_schlaegt_redigiert_fehl() {
    // Ein leerer Eintrag ist vermutlich ein Tippfehler — laut scheitern statt still übergehen.
    let fehler = PersonenConfig::from_toml_str("namen = [\"Lisa Beispiel\", \"   \"]")
        .expect_err("ein leerer Eintrag muss fehlschlagen");

    assert_eq!(fehler, ConfigError::PersonNamesInvalid);
}

#[test]
fn namen_als_einzelner_text_statt_liste_schlaegt_redigiert_fehl() {
    let fehler = PersonenConfig::from_toml_str("namen = \"Lisa Beispiel\"")
        .expect_err("ein einzelner Text statt einer Liste muss fehlschlagen");

    assert_eq!(fehler, ConfigError::PersonNamesInvalid);
    assert!(!fehler.to_string().contains("Lisa"));
}

#[test]
fn eine_kaputte_config_schlaegt_redigiert_fehl() {
    let fehler = PersonenConfig::from_toml_str("namen = [\"Lisa Beispiel\"")
        .expect_err("kaputtes TOML muss fehlschlagen");

    assert_eq!(fehler, ConfigError::Malformed(ConfigKind::Personen));
    assert!(!fehler.to_string().contains("Lisa"));
}
