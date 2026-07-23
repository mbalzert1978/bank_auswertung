//! io-Integrationstest ([Issue 0059](../../../docs/issues/0059-config-loader-toml-seed.md)):
//! der **toml-Loader des Obfuskierungs-Seeds**. `io` liest den Seed aus einer Datei und reicht ihn
//! als rohen `u64`-**Wert** in den `obfuskierung`-Slice — exakt das Muster der Fenster-Config
//! (Issue 0054), nur für den Seed. Der Slice liest weiterhin **keine** Datei; gleicher Seed →
//! gleiche `PERSON_x`/`KONTO_x` über Importe hinweg ([ADR-0010](../../../docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)).
//!
//! Getestet wird der Loader an seiner öffentlichen Oberfläche: eine gültige Fixture → der erwartete
//! Wert; eine fehlende **und** eine ungültige Config → je ein sauberer, **redigierter** Fehlschlag
//! (kein Panic, kein Rohwert — insbesondere nie der gelesene Seed — im Fehlertext).

use std::path::Path;

use io::config::{ConfigError, ConfigKind, SeedConfig};

/// Die gültige Fixture-Config, gegen die der Loader den erwarteten Seed liefern muss.
const VALID: &str = include_str!("../fixtures/obfuskierung_seed/valid.toml");

// --- Happy Path: gültige Config → erwarteter Seed ----------------------------------------

#[test]
fn eine_gueltige_config_liefert_den_erwarteten_seed() {
    let config = SeedConfig::from_toml_str(VALID).expect("gültige Config lädt");

    assert_eq!(config.seed(), 42);
}

// --- Happy Path über das echte Dateisystem: `load` liest die Fixture-Datei ---------------

#[test]
fn der_dateisystem_pfad_load_liest_die_fixture_erfolgreich() {
    // Der Produktionspfad ist `load` (fs-read + Parse), nicht `from_toml_str` — also deckt ein Test
    // ihn gegen die reale Fixture-Datei ab. Der Pfad wird crate-relativ über `CARGO_MANIFEST_DIR`
    // gebaut, damit er nicht vom Working Directory des Test-Runners abhängt.
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/obfuskierung_seed/valid.toml");
    let config = SeedConfig::load(&pfad).expect("die gültige Fixture-Datei lädt über load");

    assert_eq!(config.seed(), 42);
}

// --- Redaktion: der Seed leakt nicht über die Debug-Ausgabe ------------------------------

#[test]
fn die_debug_ausgabe_redigiert_den_seed() {
    // Der Seed ist so sensibel wie ein Klartext (ADR-0010): eine `{:?}`-Ausgabe (etwa in einer
    // Panik-Message) darf ihn nie zeigen. Ein auffälliger Seed-Wert darf im Debug-String fehlen.
    let config = SeedConfig::from_toml_str("seed = 1234567").expect("gültige Config lädt");

    assert!(!format!("{config:?}").contains("1234567"));
}

// --- Fehlende Config: sauberer, redigierter Fehlschlag -----------------------------------

#[test]
fn eine_fehlende_config_schlaegt_redigiert_fehl() {
    // Ein Pfad, unter dem keine Datei liegt — der Loader gibt ein `Result`, nie einen Panic. Der Pfad
    // wird crate-relativ über `CARGO_MANIFEST_DIR` gebaut, damit er nicht vom Working Directory des
    // Test-Runners abhängt.
    let fehlender_pfad = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/obfuskierung_seed/existiert-nicht.toml");
    let fehler =
        SeedConfig::load(&fehlender_pfad).expect_err("eine fehlende Datei muss fehlschlagen");

    // Redigiert: der Fehler nennt die statische Ursache, nie den Pfad (kein Rohwert).
    assert_eq!(fehler, ConfigError::Unreadable(ConfigKind::Seed));
    assert!(!fehler.to_string().contains("existiert-nicht"));
}

// --- Ungültiger Wert: sauberer, redigierter Fehlschlag (kein Seed im Fehler) -------------

#[test]
fn eine_config_mit_nicht_ganzzahligem_seed_schlaegt_redigiert_fehl() {
    // `seed` ist kein Integer → ungültig; der gelesene Rohwert darf im Fehler nicht auftauchen.
    let fehler = SeedConfig::from_toml_str("seed = \"geheim\"")
        .expect_err("ein nicht-ganzzahliger Seed muss fehlschlagen");

    assert_eq!(fehler, ConfigError::SeedInvalid);
    assert!(!fehler.to_string().contains("geheim"));
}

// --- Negativer Seed: kein u64, redigierter Fehlschlag (der Wert bleibt draußen) ----------

#[test]
fn eine_config_mit_negativem_seed_schlaegt_redigiert_fehl() {
    // Ein negativer Wert passt nicht in den `u64`-Seed — der Rand lehnt ab, der Rohwert bleibt draußen.
    const ROHER_SEED: &str = "-7";
    let fehler = SeedConfig::from_toml_str(&format!("seed = {ROHER_SEED}"))
        .expect_err("ein negativer Seed muss fehlschlagen");

    assert_eq!(fehler, ConfigError::SeedInvalid);
    // Redigiert: der konkrete Roh-Seed-Wert taucht nicht im Fehlertext auf. Geprüft auf genau diesen
    // Wert-String — nicht auf eine einzelne Ziffer, die auch ein künftiger statischer Fehlertext tragen
    // könnte.
    assert!(!fehler.to_string().contains(ROHER_SEED));
}

// --- Fehlender Schlüssel: eigener, redigierter Fehlschlag --------------------------------

#[test]
fn eine_config_ohne_seed_schlaegt_redigiert_fehl() {
    // Gültiges (leeres) TOML, aber der Pflichtschlüssel fehlt — abgegrenzt von „kein TOML".
    let fehler =
        SeedConfig::from_toml_str("").expect_err("ein fehlender Schlüssel muss fehlschlagen");

    assert_eq!(fehler, ConfigError::SeedMissing);
}

// --- Kaputtes TOML: eigener, redigierter Fehlschlag (kein Rohwert) -----------------------

#[test]
fn eine_kaputte_config_schlaegt_redigiert_fehl() {
    // Syntaktisch kaputtes TOML mit einem wiedererkennbaren Token: der Parsefehler darf es nicht in
    // den redigierten Fehler durchreichen (der toml-Fehler zitiert sonst gern die kaputte Zeile).
    let fehler =
        SeedConfig::from_toml_str("nichtssagend").expect_err("kaputtes TOML muss fehlschlagen");

    assert_eq!(fehler, ConfigError::Malformed(ConfigKind::Seed));
    assert!(!fehler.to_string().contains("nichtssagend"));
}

// --- Randfall seed = 0: ein gültiger Seed (anders als window_days = 0) -------------------

#[test]
fn seed_null_ist_ein_gueltiger_seed() {
    // Ein Seed von 0 ist ein legitimer Wert (keine `NonZero`-Schranke wie bei window_days).
    let config =
        SeedConfig::from_toml_str("seed = 0").expect("seed = 0 ist ein gültiger Seed, kein Fehler");

    assert_eq!(config.seed(), 0);
}
