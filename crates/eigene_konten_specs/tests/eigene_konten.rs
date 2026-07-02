//! Black-Box-Spezifikation des Eigene-Konten-Slice — nur über die public API
//! des Adapters (Fluent-`KontenTestApi` + DTOs), nie über Interna.

use eigene_konten_adapter::{KontenTestApi, LadeError, Rolle, Zuordnung};

const EIGENE_GIRO: &str = "DE20 1234 5678 0000 0011 11";
const EIGENE_HAUSHALT: &str = "DE93 1234 5678 0000 0022 22";
const EIGENE_SPAR: &str = "DE69 1234 5678 0000 0033 33";
const EIGENE_VORLAUF: &str = "DE45 1234 5678 0000 0044 44";
const FREMDE_IBAN: &str = "DE90 8765 4321 0000 0099 99";

/// Eine Fixture mit allen vier eigenen Konten.
fn vier_konten() -> eigene_konten_adapter::GeladenesRegister {
    KontenTestApi::new()
        .mit_konto(EIGENE_GIRO, "Giro")
        .mit_konto(EIGENE_HAUSHALT, "Haushalt")
        .mit_konto(EIGENE_SPAR, "Spar")
        .mit_konto(EIGENE_VORLAUF, "Vorlauf")
        .lade()
        .expect("gültige Fixture-Config lädt")
}

// --- Nachschlagen ----------------------------------------------------------

#[test]
fn eigene_giro_iban_wird_als_eigen_mit_rolle_giro_erkannt() {
    assert_eq!(
        vier_konten().zuordnung(EIGENE_GIRO).expect("gültige IBAN"),
        Zuordnung::Eigen(Rolle::Giro)
    );
}

#[test]
fn eigene_vorlauf_iban_wird_als_eigen_mit_rolle_vorlauf_erkannt() {
    assert_eq!(
        vier_konten()
            .zuordnung(EIGENE_VORLAUF)
            .expect("gültige IBAN"),
        Zuordnung::Eigen(Rolle::Vorlauf)
    );
}

#[test]
fn fremde_iban_wird_als_fremd_erkannt() {
    assert_eq!(
        vier_konten().zuordnung(FREMDE_IBAN).expect("gültige IBAN"),
        Zuordnung::Fremd
    );
}

#[test]
fn dieselbe_config_beantwortet_eigen_fremd_und_rolle() {
    let konten = vier_konten();

    assert_eq!(
        konten.zuordnung(EIGENE_SPAR).expect("gültige IBAN"),
        Zuordnung::Eigen(Rolle::Spar)
    );
    assert_eq!(
        konten.zuordnung(EIGENE_HAUSHALT).expect("gültige IBAN"),
        Zuordnung::Eigen(Rolle::Haushalt)
    );
    assert_eq!(
        konten.zuordnung(FREMDE_IBAN).expect("gültige IBAN"),
        Zuordnung::Fremd
    );
}

#[test]
fn nachschlagen_ignoriert_schreibweise_und_leerzeichen() {
    assert_eq!(
        vier_konten()
            .zuordnung("de2012345678000000 1111")
            .expect("gültige IBAN"),
        Zuordnung::Eigen(Rolle::Giro)
    );
}

// --- Ablehnung statt stillem Überspringen ----------------------------------

#[test]
fn config_mit_doppelter_iban_wird_abgelehnt() {
    let ergebnis = KontenTestApi::new()
        .mit_konto(EIGENE_GIRO, "Giro")
        .mit_konto(EIGENE_GIRO, "Spar")
        .lade();

    assert!(matches!(ergebnis, Err(LadeError::DoppelteIban)));
}

#[test]
fn config_ganz_ohne_konto_eintrag_wird_abgelehnt() {
    assert!(matches!(
        KontenTestApi::new().lade(),
        Err(LadeError::KeineKonten)
    ));
}

#[test]
fn config_mit_ungültiger_iban_wird_abgelehnt() {
    // gültige Giro-IBAN mit verfälschter Prüfziffer (20 -> 21)
    let ergebnis = KontenTestApi::new()
        .mit_konto("DE21 1234 5678 0000 0011 11", "Giro")
        .lade();

    assert!(matches!(ergebnis, Err(LadeError::Quelle(_))));
}

#[test]
fn config_mit_unbekannter_rolle_wird_abgelehnt() {
    let ergebnis = KontenTestApi::new()
        .mit_konto(EIGENE_GIRO, "Tagesgeld")
        .lade();

    assert!(matches!(ergebnis, Err(LadeError::Quelle(_))));
}
