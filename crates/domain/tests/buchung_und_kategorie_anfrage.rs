use domain::{
    Betrag, Buchung, Buchungsart, Empfänger, Fingerabdruck, Gegenkonto, GläubigerId, KontoId,
    ObfuskierterText, Pseudonym, Richtung, Währung,
};

fn firma_buchung() -> Buchung {
    let gläubiger_id = GläubigerId::new("DE98ZZZ09999999999").expect("gültige Gläubiger-ID");
    let fallback_pseudonym = Pseudonym::new("PERSON_00").expect("gültiges Pseudonym");
    let empfänger =
        Empfänger::resolve(Some(gläubiger_id), || Some(fallback_pseudonym)).expect("Firma");

    Buchung::new(
        KontoId::new("KONTO_1").expect("gültige KontoId"),
        Betrag::new(1299, Währung::Eur).expect("gültiger Betrag"),
        Richtung::Ausgang,
        empfänger,
        Gegenkonto::Fremd,
        Some(ObfuskierterText::new("Streaming-Abo").expect("nicht leer")),
        Buchungsart::Lastschrift,
    )
}

fn person_buchung_ohne_verwendungszweck() -> Buchung {
    let pseudonym = Pseudonym::new("PERSON_07").expect("gültiges Pseudonym");
    let empfänger = Empfänger::resolve(None, || Some(pseudonym)).expect("Person");

    Buchung::new(
        KontoId::new("KONTO_1").expect("gültige KontoId"),
        Betrag::new(4200, Währung::Eur).expect("gültiger Betrag"),
        Richtung::Ausgang,
        empfänger,
        Gegenkonto::Fremd,
        None,
        Buchungsart::Überweisung,
    )
}

#[test]
fn buchung_lässt_sich_aus_value_objects_konstruieren() {
    let buchung = firma_buchung();

    assert_eq!(buchung.richtung(), Richtung::Ausgang);
}

/// `Betrag` ist vorzeichenlos, das Vorzeichen lebt ausschließlich in
/// `Richtung`: es gibt keinen `Betrag`-Wert, der als negativer `Eingang`
/// verbucht werden könnte — der Versuch scheitert schon am `Betrag` selbst,
/// lange bevor eine `Buchung` überhaupt zusammengesetzt wird.
#[test]
fn negative_eingang_buchung_ist_nicht_konstruierbar() {
    assert!(Betrag::new(-1299, Währung::Eur).is_none());
}

#[test]
fn fingerabdruck_ist_eine_reine_funktion_der_buchung() {
    let first_buchung = firma_buchung();
    let second_buchung = firma_buchung();

    assert_eq!(
        Fingerabdruck::from_buchung(&first_buchung),
        Fingerabdruck::from_buchung(&second_buchung)
    );
}

#[test]
fn projektion_zu_kategorie_anfrage_trägt_fingerabdruck_und_freitext() {
    let buchung = firma_buchung();

    let anfrage = buchung.to_kategorie_anfrage();

    assert_eq!(anfrage.fingerabdruck, Fingerabdruck::from_buchung(&buchung));
    assert_eq!(
        anfrage
            .verwendungszweck
            .expect("Verwendungszweck vorhanden")
            .as_str(),
        "Streaming-Abo"
    );
}

/// Fehlt der Freitext an der `Buchung`, trägt auch die Projektion keinen — es
/// gibt keinen leeren `ObfuskierterText`, sondern `None`.
#[test]
fn projektion_ohne_verwendungszweck_trägt_keinen_freitext() {
    let buchung = person_buchung_ohne_verwendungszweck();

    let anfrage = buchung.to_kategorie_anfrage();

    assert!(anfrage.verwendungszweck.is_none());
}
