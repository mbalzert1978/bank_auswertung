use domain::{
    Betrag, GläubigerId, KontoId, NonEmptyString, ObfuskierterText, Pseudonym, Währung
};

#[test]
fn negative_betrag_is_not_constructible() {
    assert!(Betrag::new(-1, Währung::Eur).is_none());
}

#[test]
fn non_empty_string_rejects_empty_and_whitespace() {
    assert!(NonEmptyString::new("").is_none());
    assert!(NonEmptyString::new("   ").is_none());
}

#[test]
fn non_empty_string_trims_and_keeps_content() {
    let text = NonEmptyString::new("  EDEKA  ").expect("nicht leer");

    assert_eq!(text.as_str(), "EDEKA");
}

#[test]
fn valid_betrag_is_constructible() {
    let betrag = Betrag::new(4_90, Währung::Eur);

    assert!(betrag.is_some());
}

#[test]
fn empty_konto_id_is_not_constructible() {
    assert!(KontoId::new("").is_none());
    assert!(KontoId::new("   ").is_none());
}

#[test]
fn valid_konto_id_is_constructible() {
    assert!(KontoId::new("KONTO_1").is_some());
}

#[test]
fn empty_pseudonym_is_not_constructible() {
    assert!(Pseudonym::new("").is_none());
}

#[test]
fn valid_pseudonym_is_constructible() {
    assert!(Pseudonym::new("PERSON_07").is_some());
}

#[test]
fn empty_gläubiger_id_is_not_constructible() {
    assert!(GläubigerId::new("").is_none());
}

#[test]
fn valid_gläubiger_id_is_constructible() {
    assert!(GläubigerId::new("DE98ZZZ09999999999").is_some());
}

#[test]
fn empty_obfuskierter_text_is_not_constructible() {
    assert!(ObfuskierterText::new("").is_none());
    assert!(ObfuskierterText::new("   ").is_none());
}

#[test]
fn obfuskierter_text_trims_surrounding_whitespace() {
    let text = ObfuskierterText::new("  Lebensmittel  ").expect("nicht leer");

    assert_eq!(text.as_str(), "Lebensmittel");
}
