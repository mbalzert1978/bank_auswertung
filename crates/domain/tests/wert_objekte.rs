use domain::{
    Betrag, GläubigerId, KontoId, NonEmptyString, ObfuskierterText, Pseudonym, Währung
};

#[test]
fn negativer_betrag_ist_nicht_konstruierbar() {
    assert!(Betrag::new(-1, Währung::Eur).is_none());
}

#[test]
fn leerer_oder_whitespace_text_ist_nicht_konstruierbar() {
    assert!(NonEmptyString::new("").is_none());
    assert!(NonEmptyString::new("   ").is_none());
}

#[test]
fn text_wird_getrimmt_und_behält_den_inhalt() {
    let text = NonEmptyString::new("  EDEKA  ").expect("nicht leer");

    assert_eq!(text.as_str(), "EDEKA");
}

#[test]
fn gültiger_betrag_ist_konstruierbar() {
    let betrag = Betrag::new(4_90, Währung::Eur);

    assert!(betrag.is_some());
}

#[test]
fn leere_konto_id_ist_nicht_konstruierbar() {
    assert!(KontoId::new("").is_none());
    assert!(KontoId::new("   ").is_none());
}

#[test]
fn gültige_konto_id_ist_konstruierbar() {
    assert!(KontoId::new("KONTO_1").is_some());
}

#[test]
fn leeres_pseudonym_ist_nicht_konstruierbar() {
    assert!(Pseudonym::new("").is_none());
}

#[test]
fn gültiges_pseudonym_ist_konstruierbar() {
    assert!(Pseudonym::new("PERSON_07").is_some());
}

#[test]
fn leere_gläubiger_id_ist_nicht_konstruierbar() {
    assert!(GläubigerId::new("").is_none());
}

#[test]
fn gültige_gläubiger_id_ist_konstruierbar() {
    assert!(GläubigerId::new("DE98ZZZ09999999999").is_some());
}

#[test]
fn leerer_obfuskierter_text_ist_nicht_konstruierbar() {
    assert!(ObfuskierterText::new("").is_none());
    assert!(ObfuskierterText::new("   ").is_none());
}

#[test]
fn obfuskierter_text_trimmt_umgebende_leerzeichen() {
    let text = ObfuskierterText::new("  Lebensmittel  ").expect("nicht leer");

    assert_eq!(text.as_str(), "Lebensmittel");
}
