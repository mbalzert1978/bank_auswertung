//! Black-Box-Spezifikation des CSV-CAMT-v8-Slice — nur über die public API des
//! Adapters (Fluent-`CsvImportTestApi` + DTOs), nie über Interna. Die
//! Betrag-Format-Matrix und die Buchungsart-Deutung sind Algorithmen und als
//! Unittests direkt in ihren Adapter-Modulen beheimatet (nicht hier); dieser Spec
//! behält je einen repräsentativen Slice-Fehlerpfad.

use csv_camt_v8_adapter::{
    Betrag, Buchung, Buchungsart, CsvImportTestApi, Empfänger, Gegenkonto, GläubigerId,
    ImportError, KontoId, ObfuskierterText, Pseudonym, Richtung, Rolle, Umsatzzeile, Währung,
};

// --- Fixtures & Hilfen ------------------------------------------------------

const EIGENE_GIRO: &str = "DE20 1234 5678 0000 0011 11";
const EIGENE_SPAR: &str = "DE69 1234 5678 0000 0033 33";
const FREMDE_IBAN: &str = "DE90 8765 4321 0000 0099 99";

/// Eine Arrange-Stufe mit den beiden eigenen Konten (Giro + Spar).
fn api() -> CsvImportTestApi {
    CsvImportTestApi::new()
        .mit_konto(EIGENE_GIRO, "Giro")
        .mit_konto(EIGENE_SPAR, "Spar")
}

/// Eine bereits geparste Standard-Zeile (eigenes Giro, EUR, Fremd-Gegenseite,
/// Person „Anna Schmidt"); einzelne Felder überschreibt der Test per struct-update.
fn zeile_standard() -> Umsatzzeile {
    Umsatzzeile {
        auftragskonto: EIGENE_GIRO.into(),
        buchungstext: "LASTSCHRIFT".into(),
        verwendungszweck: String::new(),
        glaeubiger_id: String::new(),
        beguenstigter: "Anna Schmidt".into(),
        gegen_iban: FREMDE_IBAN.into(),
        betrag: "-1,00".into(),
        waehrung: "EUR".into(),
    }
}

fn buchung_at(buchungen: &[Buchung], n: usize) -> &Buchung {
    buchungen.get(n).expect("Buchung an erwarteter Position")
}

fn firma(gläubiger_id: &str) -> Empfänger {
    Empfänger::Firma(GläubigerId::new(gläubiger_id).expect("gültige Gläubiger-ID"))
}

fn person(token: &str) -> Empfänger {
    Empfänger::Person(Pseudonym::new(token).expect("gültiges Pseudonym"))
}

fn buchung(
    empfänger: Empfänger,
    cent: u64,
    richtung: Richtung,
    gegenkonto: Gegenkonto,
    verwendungszweck: Option<&str>,
    buchungsart: Buchungsart,
) -> Buchung {
    Buchung::new(
        KontoId::new("KONTO_1").expect("gültige KontoId"),
        Betrag::new(cent, Währung::Eur),
        richtung,
        empfänger,
        gegenkonto,
        verwendungszweck.map(|t| ObfuskierterText::new(t).expect("nicht leer")),
        buchungsart,
    )
}

/// Ein vollständiges, kohärentes Import-Szenario (sechs Zeilen), das Firma/Person,
/// wiederkehrenden Namen, eigene Umbuchung, Eingang/Ausgang, leeren
/// Verwendungszweck und alle Buchungsarten abdeckt. Alle Zeilen tragen dasselbe
/// Auftragskonto → `KONTO_1`.
fn szenario() -> Vec<Umsatzzeile> {
    vec![
        Umsatzzeile {
            buchungstext: "LASTSCHRIFT".into(),
            verwendungszweck: "Netflix Abo Dezember".into(),
            glaeubiger_id: "DE98ZZZ09999999999".into(),
            beguenstigter: "Netflix International B.V.".into(),
            betrag: "-12,99".into(),
            ..zeile_standard()
        },
        Umsatzzeile {
            buchungstext: "DAUERAUFTRAG".into(),
            verwendungszweck: "Uebertrag Sparen".into(),
            beguenstigter: "Max Mustermann".into(),
            gegen_iban: EIGENE_SPAR.into(),
            betrag: "-250,00".into(),
            ..zeile_standard()
        },
        Umsatzzeile {
            buchungstext: "ÜBERWEISUNG".into(),
            verwendungszweck: "Miete Anteil".into(),
            beguenstigter: "Anna Schmidt".into(),
            betrag: "-1.234,56".into(),
            ..zeile_standard()
        },
        Umsatzzeile {
            buchungstext: "ÜBERWEISUNG".into(),
            verwendungszweck: String::new(),
            beguenstigter: "Anna Schmidt".into(),
            betrag: "-15,00".into(),
            ..zeile_standard()
        },
        Umsatzzeile {
            buchungstext: "GUTSCHRIFT".into(),
            verwendungszweck: "Erstattung Abschlag".into(),
            glaeubiger_id: "DE11ZZZ00000000001".into(),
            beguenstigter: "Stadtwerke Musterstadt".into(),
            betrag: "19,90".into(),
            ..zeile_standard()
        },
        Umsatzzeile {
            buchungstext: "ENTGELT".into(),
            verwendungszweck: "Kontofuehrungsentgelt".into(),
            beguenstigter: "Sparkasse Musterstadt".into(),
            betrag: "-4,90".into(),
            ..zeile_standard()
        },
    ]
}

fn importiere(zeilen: Vec<Umsatzzeile>) -> Vec<Buchung> {
    api()
        .mit_zeilen(zeilen)
        .importiere()
        .expect("Szenario importiert")
}

// --- Der vollständige Pfad Zeilen → Buchung-Liste --------------------------

#[test]
fn zeilen_werden_vollständig_und_obfuskiert_zu_buchungen_gemappt() {
    let buchungen = importiere(szenario());

    let erwartet = vec![
        buchung(
            firma("DE98ZZZ09999999999"),
            1299,
            Richtung::Ausgang,
            Gegenkonto::Fremd,
            Some("Netflix Abo Dezember"),
            Buchungsart::Lastschrift,
        ),
        buchung(
            person("PERSON_1"),
            25000,
            Richtung::Ausgang,
            Gegenkonto::Eigen(Rolle::Spar),
            Some("Uebertrag Sparen"),
            Buchungsart::Dauerauftrag,
        ),
        buchung(
            person("PERSON_2"),
            123_456,
            Richtung::Ausgang,
            Gegenkonto::Fremd,
            Some("Miete Anteil"),
            Buchungsart::Überweisung,
        ),
        buchung(
            person("PERSON_2"),
            1500,
            Richtung::Ausgang,
            Gegenkonto::Fremd,
            None,
            Buchungsart::Überweisung,
        ),
        buchung(
            firma("DE11ZZZ00000000001"),
            1990,
            Richtung::Eingang,
            Gegenkonto::Fremd,
            Some("Erstattung Abschlag"),
            Buchungsart::Sonstige,
        ),
        buchung(
            person("PERSON_3"),
            490,
            Richtung::Ausgang,
            Gegenkonto::Fremd,
            Some("Kontofuehrungsentgelt"),
            Buchungsart::Entgelt,
        ),
    ];
    assert_eq!(buchungen, erwartet);
}

// --- Einzelne Acceptance-Kriterien, fokussiert -----------------------------

#[test]
fn auftragskonto_wird_zu_obfuskierter_konto_id() {
    // Beide Zeilen tragen dieselbe eigene IBAN → dasselbe KONTO_1 (nie die IBAN).
    let buchungen = importiere(vec![
        Umsatzzeile {
            verwendungszweck: "A".into(),
            glaeubiger_id: "DE98ZZZ09999999999".into(),
            betrag: "-1,00".into(),
            ..zeile_standard()
        },
        Umsatzzeile {
            verwendungszweck: "B".into(),
            glaeubiger_id: "DE98ZZZ09999999999".into(),
            betrag: "-2,00".into(),
            ..zeile_standard()
        },
    ]);

    assert_eq!(
        buchungen,
        vec![
            buchung(
                firma("DE98ZZZ09999999999"),
                100,
                Richtung::Ausgang,
                Gegenkonto::Fremd,
                Some("A"),
                Buchungsart::Lastschrift,
            ),
            buchung(
                firma("DE98ZZZ09999999999"),
                200,
                Richtung::Ausgang,
                Gegenkonto::Fremd,
                Some("B"),
                Buchungsart::Lastschrift,
            ),
        ]
    );
}

#[test]
fn gegen_iban_eines_eigenen_kontos_wird_zu_eigen_mit_rolle() {
    // Umbuchung Giro -> eigenes Spar-Konto.
    let buchungen = importiere(vec![Umsatzzeile {
        buchungstext: "DAUERAUFTRAG".into(),
        verwendungszweck: "Sparen".into(),
        beguenstigter: "Markus Mustermann".into(),
        gegen_iban: EIGENE_SPAR.into(),
        betrag: "-100,00".into(),
        ..zeile_standard()
    }]);

    assert_eq!(
        buchungen,
        vec![buchung(
            person("PERSON_1"),
            10000,
            Richtung::Ausgang,
            Gegenkonto::Eigen(Rolle::Spar),
            Some("Sparen"),
            Buchungsart::Dauerauftrag,
        )]
    );
}

#[test]
fn fremde_gegen_iban_wird_zu_fremd() {
    let buchungen = importiere(vec![Umsatzzeile {
        verwendungszweck: "Abo".into(),
        glaeubiger_id: "DE98ZZZ09999999999".into(),
        betrag: "-9,99".into(),
        ..zeile_standard()
    }]);

    assert_eq!(
        buchungen,
        vec![buchung(
            firma("DE98ZZZ09999999999"),
            999,
            Richtung::Ausgang,
            Gegenkonto::Fremd,
            Some("Abo"),
            Buchungsart::Lastschrift,
        )]
    );
}

#[test]
fn zwei_zeilen_gleicher_klartextname_ergeben_dasselbe_pseudonym() {
    let buchungen = importiere(vec![
        Umsatzzeile {
            verwendungszweck: "erst".into(),
            beguenstigter: "Anna Schmidt".into(),
            betrag: "-1,00".into(),
            ..zeile_standard()
        },
        Umsatzzeile {
            verwendungszweck: "nochmal".into(),
            beguenstigter: "Anna Schmidt".into(),
            betrag: "-2,00".into(),
            ..zeile_standard()
        },
    ]);

    let erste = buchung_at(&buchungen, 0);
    let zweite = buchung_at(&buchungen, 1);
    assert_eq!(erste.empfänger(), zweite.empfänger());
    assert_eq!(erste.empfänger(), &person("PERSON_1"));
}

#[test]
fn empfänger_mit_gläubiger_id_wird_firma() {
    let buchungen = importiere(vec![Umsatzzeile {
        glaeubiger_id: "DE98ZZZ09999999999".into(),
        beguenstigter: "Netflix International B.V.".into(),
        betrag: "-12,99".into(),
        ..zeile_standard()
    }]);

    assert_eq!(
        buchung_at(&buchungen, 0).empfänger(),
        &firma("DE98ZZZ09999999999")
    );
}

#[test]
fn empfänger_ohne_gläubiger_id_wird_fail_safe_person() {
    // Kein Gläubiger-ID, aber ein Name → im Zweifel Person (Klarname nicht leaken).
    let buchungen = importiere(vec![Umsatzzeile {
        buchungstext: "ENTGELT".into(),
        verwendungszweck: "Gebuehr".into(),
        beguenstigter: "Sparkasse Musterstadt".into(),
        betrag: "-4,90".into(),
        ..zeile_standard()
    }]);

    assert_eq!(buchung_at(&buchungen, 0).empfänger(), &person("PERSON_1"));
}

#[test]
fn leerer_verwendungszweck_wird_none() {
    let buchungen = importiere(vec![
        Umsatzzeile {
            verwendungszweck: String::new(),
            betrag: "-1,00".into(),
            ..zeile_standard()
        },
        Umsatzzeile {
            verwendungszweck: "Netflix Abo Dezember".into(),
            glaeubiger_id: "DE98ZZZ09999999999".into(),
            betrag: "-2,00".into(),
            ..zeile_standard()
        },
    ]);

    assert!(buchung_at(&buchungen, 0).verwendungszweck().is_none());
    assert_eq!(
        buchung_at(&buchungen, 1)
            .verwendungszweck()
            .map(ObfuskierterText::as_str),
        Some("Netflix Abo Dezember")
    );
}

// --- Der tragende Datenschutz-Test -----------------------------------------

/// Rohe strukturelle Identifikatoren, die im Szenario vorkommen — gespacet (wie im
/// CSV) und normalisiert.
const STRUKTURELLE_IDENTIFIKATOREN: &[&str] = &[
    "DE20 1234 5678 0000 0011 11",
    "DE20123456780000001111",
    "DE69 1234 5678 0000 0033 33",
    "DE69123456780000003333",
    "DE90 8765 4321 0000 0099 99",
    "DE90876543210000009999",
];

#[test]
fn kein_roher_struktureller_identifikator_im_output() {
    let buchungen = importiere(szenario());

    let haystack = format!("{buchungen:?}");
    for raw in STRUKTURELLE_IDENTIFIKATOREN {
        assert!(
            !haystack.contains(raw),
            "roher struktureller Identifikator {raw} im Adapter-Output geleakt"
        );
    }
}

// --- Fehlerpfade: lautes Scheitern statt Falschsummierung ------------------

#[test]
fn gemischte_währung_ist_ein_fehler() {
    let result = api()
        .mit_zeile(Umsatzzeile {
            betrag: "-1,00".into(),
            ..zeile_standard()
        })
        .mit_zeile(Umsatzzeile {
            betrag: "-2,00".into(),
            waehrung: "USD".into(),
            ..zeile_standard()
        })
        .importiere();

    assert!(matches!(result, Err(ImportError::MixedWährung(_, _))));
}

#[test]
fn unbekannte_währung_ist_ein_fehler() {
    let result = api()
        .mit_zeile(Umsatzzeile {
            waehrung: "USD".into(),
            ..zeile_standard()
        })
        .importiere();

    assert!(matches!(result, Err(ImportError::UnknownWährung(_))));
}

#[test]
fn auftragskonto_ausserhalb_der_config_ist_ein_fehler() {
    // Auftragskonto ist eine (gültige) Fremd-IBAN, nicht in der Config.
    let result = api()
        .mit_zeile(Umsatzzeile {
            auftragskonto: FREMDE_IBAN.into(),
            ..zeile_standard()
        })
        .importiere();

    assert!(matches!(result, Err(ImportError::AuftragskontoUnknown)));
}

#[test]
fn ungültiges_auftragskonto_ist_ein_fehler() {
    let result = api()
        .mit_zeile(Umsatzzeile {
            auftragskonto: "DE00 0000".into(),
            ..zeile_standard()
        })
        .importiere();

    assert!(matches!(result, Err(ImportError::AuftragskontoInvalid(_))));
}

#[test]
fn ungültiger_betrag_ist_ein_fehler() {
    let result = api()
        .mit_zeile(Umsatzzeile {
            betrag: "12,ab".into(),
            ..zeile_standard()
        })
        .importiere();

    assert!(matches!(result, Err(ImportError::BetragInvalid(_))));
}

#[test]
fn fehlender_empfänger_ist_ein_fehler() {
    // Weder Gläubiger-ID noch Name → nicht identifizierbar.
    let result = api()
        .mit_zeile(Umsatzzeile {
            buchungstext: "ENTGELT".into(),
            verwendungszweck: "Gebuehr".into(),
            beguenstigter: String::new(),
            gegen_iban: String::new(),
            betrag: "-4,90".into(),
            ..zeile_standard()
        })
        .importiere();

    assert!(matches!(result, Err(ImportError::EmpfängerMissing)));
}

#[test]
fn import_ganz_ohne_zeile_ergibt_keine_buchungen() {
    let result = api().importiere().expect("leerer Import ist gültig");

    assert!(result.is_empty());
}

// --- Datenschutz in Fehlermeldungen ----------------------------------------

#[test]
fn fehlermeldung_enthält_keine_rohe_eingabe() {
    // Ein ungültiges Auftragskonto darf im Klartext nicht in der Fehlermeldung
    // stehen — sonst leakt ein (fast-)struktureller Identifikator über Logs.
    let raw = "DE00 1234 5678";
    let error = api()
        .mit_zeile(Umsatzzeile {
            auftragskonto: raw.into(),
            ..zeile_standard()
        })
        .importiere()
        .expect_err("ungültiges Auftragskonto");

    let message = error.to_string();
    assert!(
        !message.contains(raw) && !message.contains("DE0012345678"),
        "rohe Eingabe in Fehlermeldung geleakt: {message}"
    );
}

#[test]
fn fehlermeldung_zu_fremdem_auftragskonto_leakt_die_iban_nicht() {
    let error = api()
        .mit_zeile(Umsatzzeile {
            auftragskonto: FREMDE_IBAN.into(),
            ..zeile_standard()
        })
        .importiere()
        .expect_err("Auftragskonto nicht in Config");

    let message = error.to_string();
    assert!(
        !message.contains(FREMDE_IBAN) && !message.contains("DE90876543210000009999"),
        "IBAN in Fehlermeldung geleakt: {message}"
    );
}
