//! Black-Box-Spezifikation des Obfuskierungs-Slice — nur über die public API des
//! Adapters (Fluent-`ObfuskierungTestApi` + DTOs), nie über Interna.

use obfuskierung_adapter::{Iban, ObfuskierungTestApi};

const EIGENE_GIRO: &str = "DE20 1234 5678 0000 0011 11";
const EIGENE_HAUSHALT: &str = "DE93 1234 5678 0000 0022 22";
const EIGENE_SPAR: &str = "DE69 1234 5678 0000 0033 33";

/// Baut das IBAN-Eingangs-DTO — normalisiert nur (keine Validierung mehr, siehe
/// `iban.rs`); eine leere Eingabe ergäbe `None`.
fn iban(text: &str) -> Iban {
    Iban::new(text).expect("nicht-leere Fixture-IBAN")
}

// --- Konto-Token: First-Seen & Laufstabilität ------------------------------

#[test]
fn dieselbe_iban_erhält_im_lauf_dasselbe_konto_token() {
    let mut obfuskierer = ObfuskierungTestApi::new().obfuskierer();

    let erst = obfuskierer.obfuskiere_konto(&iban(EIGENE_GIRO));
    let nochmal = obfuskierer.obfuskiere_konto(&iban(EIGENE_GIRO));

    assert_eq!(erst, nochmal);
    assert_eq!(erst.as_str(), "KONTO_1");
}

#[test]
fn verschiedene_ibans_erhalten_aufsteigende_konto_token() {
    let mut obfuskierer = ObfuskierungTestApi::new().obfuskierer();

    assert_eq!(
        obfuskierer.obfuskiere_konto(&iban(EIGENE_GIRO)).as_str(),
        "KONTO_1"
    );
    assert_eq!(
        obfuskierer
            .obfuskiere_konto(&iban(EIGENE_HAUSHALT))
            .as_str(),
        "KONTO_2"
    );
    assert_eq!(
        obfuskierer.obfuskiere_konto(&iban(EIGENE_SPAR)).as_str(),
        "KONTO_3"
    );
}

#[test]
fn schreibweise_und_leerzeichen_ändern_das_konto_token_nicht() {
    let mut obfuskierer = ObfuskierungTestApi::new().obfuskierer();

    let mit_leerzeichen = obfuskierer.obfuskiere_konto(&iban(EIGENE_GIRO));
    let andere_schreibweise = obfuskierer.obfuskiere_konto(&iban("de2012345678000000 1111"));

    assert_eq!(mit_leerzeichen, andere_schreibweise);
}

// --- Personen-Token: First-Seen & leerer Name ------------------------------

#[test]
fn derselbe_name_erhält_im_lauf_dasselbe_personen_token() {
    let mut obfuskierer = ObfuskierungTestApi::new().obfuskierer();

    let erst = obfuskierer
        .obfuskiere_person("Erika Mustermann")
        .expect("nicht-leerer Name ergibt ein Pseudonym");
    let nochmal = obfuskierer
        .obfuskiere_person("Erika Mustermann")
        .expect("nicht-leerer Name ergibt ein Pseudonym");

    assert_eq!(erst, nochmal);
    assert_eq!(erst.as_str(), "PERSON_1");
}

#[test]
fn verschiedene_namen_erhalten_aufsteigende_personen_token() {
    let mut obfuskierer = ObfuskierungTestApi::new().obfuskierer();

    assert_eq!(
        obfuskierer
            .obfuskiere_person("Erika Mustermann")
            .expect("Name ergibt Pseudonym")
            .as_str(),
        "PERSON_1"
    );
    assert_eq!(
        obfuskierer
            .obfuskiere_person("Max Beispiel")
            .expect("Name ergibt Pseudonym")
            .as_str(),
        "PERSON_2"
    );
}

#[test]
fn leerer_name_ergibt_kein_pseudonym() {
    let mut obfuskierer = ObfuskierungTestApi::new().obfuskierer();

    assert_eq!(obfuskierer.obfuskiere_person("   "), None);
}

#[test]
fn konto_und_personen_token_zählen_unabhängig() {
    let mut obfuskierer = ObfuskierungTestApi::new().obfuskierer();

    assert_eq!(
        obfuskierer.obfuskiere_konto(&iban(EIGENE_GIRO)).as_str(),
        "KONTO_1"
    );
    assert_eq!(
        obfuskierer
            .obfuskiere_person("Erika Mustermann")
            .expect("Name ergibt Pseudonym")
            .as_str(),
        "PERSON_1"
    );
}

// --- Freitext-Engpass ------------------------------------------------------

#[test]
fn freitext_wird_getrimmt_und_bleibt_inhaltlich_erhalten() {
    let obfuskierer = ObfuskierungTestApi::new().obfuskierer();

    let text = obfuskierer
        .obfuskiere_text("  Miete Juli  ")
        .expect("nicht-leerer Freitext bleibt erhalten");

    assert_eq!(text.as_str(), "Miete Juli");
}

#[test]
fn leerer_freitext_ergibt_keinen_text() {
    let obfuskierer = ObfuskierungTestApi::new().obfuskierer();

    assert_eq!(obfuskierer.obfuskiere_text("   "), None);
}

// --- Laufübergreifende Stabilität (persistiert über den Store) -------------

#[test]
fn konto_token_bleibt_über_den_lauf_hinaus_stabil() {
    let mut api = ObfuskierungTestApi::new();

    let mut erster_lauf = api.obfuskierer();
    let vorher = erster_lauf.obfuskiere_konto(&iban(EIGENE_GIRO));
    api.sichern(&erster_lauf);

    let mut zweiter_lauf = api.obfuskierer();
    let nachher = zweiter_lauf.obfuskiere_konto(&iban(EIGENE_GIRO));

    assert_eq!(vorher, nachher);
}

#[test]
fn personen_token_bleibt_über_den_lauf_hinaus_stabil() {
    let mut api = ObfuskierungTestApi::new();

    let mut erster_lauf = api.obfuskierer();
    let vorher = erster_lauf
        .obfuskiere_person("Erika Mustermann")
        .expect("Name ergibt Pseudonym");
    api.sichern(&erster_lauf);

    let mut zweiter_lauf = api.obfuskierer();
    let nachher = zweiter_lauf
        .obfuskiere_person("Erika Mustermann")
        .expect("Name ergibt Pseudonym");

    assert_eq!(vorher, nachher);
}

#[test]
fn neue_iban_nach_persistenz_zählt_die_nummerierung_weiter() {
    let mut api = ObfuskierungTestApi::new();

    let mut erster_lauf = api.obfuskierer();
    erster_lauf.obfuskiere_konto(&iban(EIGENE_GIRO)); // KONTO_1
    api.sichern(&erster_lauf);

    let mut zweiter_lauf = api.obfuskierer();
    // bekannte IBAN bleibt KONTO_1, eine neue zählt weiter statt zurückzuspringen
    assert_eq!(
        zweiter_lauf.obfuskiere_konto(&iban(EIGENE_GIRO)).as_str(),
        "KONTO_1"
    );
    assert_eq!(
        zweiter_lauf
            .obfuskiere_konto(&iban(EIGENE_HAUSHALT))
            .as_str(),
        "KONTO_2"
    );
}
