use domain::{Buchungsart, Empfänger, Gegenkonto, GläubigerId, Pseudonym, Richtung, Rolle};

fn richtung_as_sign(richtung: Richtung) -> i8 {
    match richtung {
        Richtung::Eingang => 1,
        Richtung::Ausgang => -1,
    }
}

#[test]
fn richtung_deckt_eingang_und_ausgang_erschöpfend_ab() {
    assert_eq!(richtung_as_sign(Richtung::Eingang), 1);
    assert_eq!(richtung_as_sign(Richtung::Ausgang), -1);
}

fn gegenkonto_is_eigen(gegenkonto: Gegenkonto) -> bool {
    match gegenkonto {
        Gegenkonto::Eigen(_) => true,
        Gegenkonto::Fremd => false,
    }
}

#[test]
fn gegenkonto_deckt_eigen_und_fremd_erschöpfend_ab() {
    assert!(gegenkonto_is_eigen(Gegenkonto::Eigen(Rolle::Giro)));
    assert!(!gegenkonto_is_eigen(Gegenkonto::Fremd));
}

/// `Gegenkonto::Eigen` trägt die Konto-Rolle des getroffenen eigenen Kontos —
/// „eigen ohne Rolle" ist unkonstruierbar, und zwei Umbuchungen auf Konten mit
/// verschiedener Rolle sind unterscheidbar.
#[test]
fn eigenes_gegenkonto_trägt_seine_rolle() {
    assert_eq!(
        Gegenkonto::Eigen(Rolle::Spar),
        Gegenkonto::Eigen(Rolle::Spar)
    );
    assert_ne!(
        Gegenkonto::Eigen(Rolle::Spar),
        Gegenkonto::Eigen(Rolle::Vorlauf)
    );
}

fn buchungsart_description(buchungsart: Buchungsart) -> &'static str {
    match buchungsart {
        Buchungsart::Lastschrift => "Lastschrift",
        Buchungsart::Dauerauftrag => "Dauerauftrag",
        Buchungsart::Überweisung => "Überweisung",
        Buchungsart::Entgelt => "Entgelt",
        Buchungsart::Sonstige => "Sonstige",
    }
}

#[test]
fn buchungsart_deckt_alle_varianten_erschöpfend_ab() {
    assert_eq!(buchungsart_description(Buchungsart::Entgelt), "Entgelt");
    assert_eq!(buchungsart_description(Buchungsart::Sonstige), "Sonstige");
}

#[test]
fn empfänger_mit_gläubiger_id_wird_als_firma_aufgelöst() {
    let gläubiger_id = GläubigerId::new("DE98ZZZ09999999999").expect("gültige Gläubiger-ID");
    let fallback_pseudonym = Pseudonym::new("PERSON_00").expect("gültiges Pseudonym");

    let empfänger = Empfänger::resolve(Some(gläubiger_id.clone()), || Some(fallback_pseudonym))
        .expect("Firma ist auflösbar");

    assert_eq!(empfänger, Empfänger::Firma(gläubiger_id));
}

#[test]
fn empfänger_ohne_gläubiger_id_wird_fail_safe_als_person_aufgelöst() {
    let pseudonym = Pseudonym::new("PERSON_07").expect("gültiges Pseudonym");

    let empfänger =
        Empfänger::resolve(None, || Some(pseudonym.clone())).expect("Person ist auflösbar");

    assert_eq!(empfänger, Empfänger::Person(pseudonym));
}

/// Ohne Gläubiger-ID **und** ohne Pseudonym (kein Name) ist die Gegenpartei
/// nicht identifizierbar — die fail-safe Auflösung ergibt dann `None`.
#[test]
fn empfänger_ohne_gläubiger_id_und_ohne_pseudonym_ist_nicht_auflösbar() {
    assert!(Empfänger::resolve(None, || None).is_none());
}
