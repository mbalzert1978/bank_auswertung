use std::path::{Path, PathBuf};

use csv_camt_v8::{CsvCamtV8, CsvQuelle, CsvQuelleError, ImportError, Umsatzquelle};
use domain::{
    Betrag, Buchung, Buchungsart, Empfänger, Gegenkonto, GläubigerId, KontoId, ObfuskierterText,
    Pseudonym, Richtung, Rolle, Währung,
};
use eigene_konten::EigeneKonten;

// --- Test-Adapter für den CsvQuelle-Port -----------------------------------

/// Datei-basierte Quelle. `std::fs` lebt hier im Testcode (am Rand) — der Crate
/// selbst bleibt fs-frei.
struct FileSource(PathBuf);

impl CsvQuelle for FileSource {
    fn read(&self) -> Result<String, CsvQuelleError> {
        Ok(std::fs::read_to_string(&self.0)?)
    }
}

/// In-Memory-Quelle — belegt, dass der Adapter nicht an eine Datei gebunden ist.
struct StringSource(String);

impl CsvQuelle for StringSource {
    fn read(&self) -> Result<String, CsvQuelleError> {
        Ok(self.0.clone())
    }
}

// --- Fixtures & Hilfen ------------------------------------------------------

const EIGENE_GIRO: &str = "DE20 1234 5678 0000 0011 11";
const EIGENE_SPAR: &str = "DE69 1234 5678 0000 0033 33";
const FREMDE_IBAN: &str = "DE90 8765 4321 0000 0099 99";

fn konten() -> EigeneKonten {
    EigeneKonten::from_toml(include_str!("fixtures/eigene-konten.dummy.toml"))
        .expect("Test-Config lädt")
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Zugriff ausschließlich über den `Umsatzquelle`-Port (Black-Box am DTO-Vertrag).
fn load(source: &impl Umsatzquelle) -> Result<Vec<Buchung>, ImportError> {
    source.buchungen()
}

fn import_fixture(name: &str) -> Vec<Buchung> {
    let adapter = CsvCamtV8::new(FileSource(fixture_path(name)), konten());
    load(&adapter).expect("Fixture importiert")
}

fn buchung_at(buchungen: &[Buchung], n: usize) -> &Buchung {
    buchungen.get(n).expect("Buchung an erwarteter Position")
}

/// Eine Inline-CSV-Zeile: nur die relevanten Felder werden gesetzt, der Rest
/// kommt aus [`Row::standard`] (eigenes Giro-Konto, EUR, Fremd-Gegenseite).
struct Row<'a> {
    auftragskonto: &'a str,
    buchungstext: &'a str,
    verwendungszweck: &'a str,
    glaeubiger: &'a str,
    beguenstigter: &'a str,
    gegen_iban: &'a str,
    betrag: &'a str,
    waehrung: &'a str,
}

impl Row<'_> {
    fn standard() -> Self {
        Self {
            auftragskonto: EIGENE_GIRO,
            buchungstext: "LASTSCHRIFT",
            verwendungszweck: "",
            glaeubiger: "",
            beguenstigter: "Anna Schmidt",
            gegen_iban: FREMDE_IBAN,
            betrag: "-1,00",
            waehrung: "EUR",
        }
    }

    /// Rendert die Zeile in Header-Reihenfolge (17 Spalten); ungenutzte leer.
    fn render(&self) -> String {
        [
            self.auftragskonto,
            "01.01.2026",
            "01.01.2026",
            self.buchungstext,
            self.verwendungszweck,
            self.glaeubiger,
            "",
            "",
            "",
            "",
            "",
            self.beguenstigter,
            self.gegen_iban,
            "",
            self.betrag,
            self.waehrung,
            "",
        ]
        .join(";")
    }
}

const HEADER: &str = "Auftragskonto;Buchungstag;Valutadatum;Buchungstext;Verwendungszweck;\
Glaeubiger ID;Mandatsreferenz;Kundenreferenz (End-to-End);Sammlerreferenz;\
Lastschrift Ursprungsbetrag;Auslagenersatz Ruecklastschrift;Beguenstigter/Zahlungspflichtiger;\
Kontonummer/IBAN;BIC (SWIFT-Code);Betrag;Waehrung;Info";

fn import_inline(rows: &[Row]) -> Result<Vec<Buchung>, ImportError> {
    let mut content = String::from(HEADER);
    for row in rows {
        content.push('\n');
        content.push_str(&row.render());
    }
    load(&CsvCamtV8::new(StringSource(content), konten()))
}

// --- Erwartete Buchungen ----------------------------------------------------

fn buchung(
    empfänger: Empfänger,
    cent: i64,
    richtung: Richtung,
    gegenkonto: Gegenkonto,
    verwendungszweck: Option<&str>,
    buchungsart: Buchungsart,
) -> Buchung {
    Buchung::new(
        KontoId::new("KONTO_1").expect("gültige KontoId"),
        Betrag::new(cent, Währung::Eur).expect("gültiger Betrag"),
        richtung,
        empfänger,
        gegenkonto,
        verwendungszweck.map(|t| ObfuskierterText::new(t).expect("nicht leer")),
        buchungsart,
    )
}

fn firma(gläubiger_id: &str) -> Empfänger {
    Empfänger::Firma(GläubigerId::new(gläubiger_id).expect("gültige Gläubiger-ID"))
}

fn person(token: &str) -> Empfänger {
    Empfänger::Person(Pseudonym::new(token).expect("gültiges Pseudonym"))
}

fn expected_buchungen() -> Vec<Buchung> {
    vec![
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
    ]
}

// --- Der vollständige Pfad CSV-Datei → Buchung-Liste -----------------------

#[test]
fn csv_datei_wird_vollständig_und_obfuskiert_zu_buchungen_gemappt() {
    let buchungen = import_fixture("umsaetze.csv");

    assert_eq!(buchungen, expected_buchungen());
}

// --- Einzelne Acceptance-Kriterien, fokussiert -----------------------------

#[test]
fn auftragskonto_wird_zu_obfuskierter_konto_id() {
    // Beide Zeilen tragen dieselbe eigene IBAN → dasselbe KONTO_1 (nie die IBAN);
    // die Voll-Gleichheit gegen KONTO_1 belegt die Auflösung.
    let buchungen = import_inline(&[
        Row {
            verwendungszweck: "A",
            glaeubiger: "DE98ZZZ09999999999",
            betrag: "-1,00",
            ..Row::standard()
        },
        Row {
            verwendungszweck: "B",
            glaeubiger: "DE98ZZZ09999999999",
            betrag: "-2,00",
            ..Row::standard()
        },
    ])
    .expect("importiert");

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
    let buchungen = import_inline(&[Row {
        buchungstext: "DAUERAUFTRAG",
        verwendungszweck: "Sparen",
        beguenstigter: "Markus Mustermann",
        gegen_iban: EIGENE_SPAR,
        betrag: "-100,00",
        ..Row::standard()
    }])
    .expect("importiert");

    let expected = buchung(
        person("PERSON_1"),
        10000,
        Richtung::Ausgang,
        Gegenkonto::Eigen(Rolle::Spar),
        Some("Sparen"),
        Buchungsart::Dauerauftrag,
    );
    assert_eq!(buchungen, vec![expected]);
}

#[test]
fn fremde_gegen_iban_wird_zu_fremd() {
    let buchungen = import_inline(&[Row {
        verwendungszweck: "Abo",
        glaeubiger: "DE98ZZZ09999999999",
        betrag: "-9,99",
        ..Row::standard()
    }])
    .expect("importiert");

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
    let buchungen = import_fixture("umsaetze.csv");

    // Zeile 3 und 4 sind beide „Anna Schmidt" (ohne Gläubiger-ID).
    let third = buchung_at(&buchungen, 2);
    let fourth = buchung_at(&buchungen, 3);
    assert_eq!(third.empfänger(), fourth.empfänger());
    assert_eq!(third.empfänger(), &person("PERSON_2"));
}

#[test]
fn empfänger_mit_gläubiger_id_wird_firma() {
    let buchungen = import_fixture("umsaetze.csv");

    assert_eq!(
        buchung_at(&buchungen, 0).empfänger(),
        &firma("DE98ZZZ09999999999")
    );
}

#[test]
fn empfänger_ohne_gläubiger_id_wird_fail_safe_person() {
    let buchungen = import_fixture("umsaetze.csv");

    // Der Entgelt-Empfänger „Sparkasse Musterstadt" trägt keine Gläubiger-ID →
    // fail-safe pseudonymisiert (im Zweifel Person, nicht Klarname leaken).
    assert_eq!(buchung_at(&buchungen, 5).empfänger(), &person("PERSON_3"));
}

#[test]
fn leerer_verwendungszweck_wird_none() {
    let buchungen = import_fixture("umsaetze.csv");

    assert!(buchung_at(&buchungen, 3).verwendungszweck().is_none());
    assert_eq!(
        buchung_at(&buchungen, 0)
            .verwendungszweck()
            .map(ObfuskierterText::as_str),
        Some("Netflix Abo Dezember")
    );
}

// --- Der tragende Datenschutz-Test -----------------------------------------

/// Rohe strukturelle Identifikatoren, die in der Fixture vorkommen — in
/// gespaceter (wie im CSV) und normalisierter Form, plus die BICs.
const STRUKTURELLE_IDENTIFIKATOREN: &[&str] = &[
    "DE20 1234 5678 0000 0011 11",
    "DE20123456780000001111",
    "DE69 1234 5678 0000 0033 33",
    "DE69123456780000003333",
    "DE90 8765 4321 0000 0099 99",
    "DE90876543210000009999",
    "ABNANL2AXXX",
    "GENODEF1S01",
];

#[test]
fn kein_roher_struktureller_identifikator_im_output() {
    let buchungen = import_fixture("umsaetze.csv");
    let anfragen: Vec<_> = buchungen
        .iter()
        .map(Buchung::to_kategorie_anfrage)
        .collect();

    let haystack = format!("{buchungen:?}{anfragen:?}");
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
    let result = import_inline(&[
        Row {
            betrag: "-1,00",
            ..Row::standard()
        },
        Row {
            betrag: "-2,00",
            waehrung: "USD",
            ..Row::standard()
        },
    ]);

    assert!(matches!(result, Err(ImportError::MixedWährung(_, _))));
}

#[test]
fn unbekannte_währung_ist_ein_fehler() {
    let result = import_inline(&[Row {
        waehrung: "USD",
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::UnknownWährung(_))));
}

#[test]
fn auftragskonto_ausserhalb_der_config_ist_ein_fehler() {
    // Auftragskonto ist eine (gültige) Fremd-IBAN, nicht in der Config.
    let result = import_inline(&[Row {
        auftragskonto: FREMDE_IBAN,
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::AuftragskontoUnknown)));
}

#[test]
fn ungültiges_auftragskonto_ist_ein_fehler() {
    let result = import_inline(&[Row {
        auftragskonto: "DE00 0000",
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::AuftragskontoInvalid(_))));
}

#[test]
fn ungültiger_betrag_ist_ein_fehler() {
    let result = import_inline(&[Row {
        betrag: "12,ab",
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::BetragInvalid(_))));
}

#[test]
fn fehlender_empfänger_ist_ein_fehler() {
    // Weder Gläubiger-ID noch Name → nicht identifizierbar.
    let result = import_inline(&[Row {
        buchungstext: "ENTGELT",
        verwendungszweck: "Gebuehr",
        beguenstigter: "",
        gegen_iban: "",
        betrag: "-4,90",
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::EmpfängerMissing)));
}

#[test]
fn datei_nur_mit_kopfzeile_ergibt_keine_buchungen() {
    let result = import_inline(&[]).expect("leere Datei ist gültig");

    assert!(result.is_empty());
}

// --- Betrag-Formate & Härtung ----------------------------------------------

#[test]
fn deutsche_betragsformate_werden_korrekt_geparst() {
    // `+`/kein Vorzeichen → Eingang; ganzzahliger Betrag ohne Komma; ein
    // Tausenderpunkt mit nur einer Nachkommastelle. Alle drei Zeilen sind
    // dieselbe „Anna Schmidt" → PERSON_1.
    let buchungen = import_inline(&[
        Row {
            verwendungszweck: "plus",
            betrag: "+5,00",
            ..Row::standard()
        },
        Row {
            verwendungszweck: "ganz",
            betrag: "1234",
            ..Row::standard()
        },
        Row {
            verwendungszweck: "eine-nachkommastelle",
            betrag: "-1.234,5",
            ..Row::standard()
        },
    ])
    .expect("importiert");

    assert_eq!(
        buchungen,
        vec![
            buchung(
                person("PERSON_1"),
                500,
                Richtung::Eingang,
                Gegenkonto::Fremd,
                Some("plus"),
                Buchungsart::Lastschrift,
            ),
            buchung(
                person("PERSON_1"),
                123_400,
                Richtung::Eingang,
                Gegenkonto::Fremd,
                Some("ganz"),
                Buchungsart::Lastschrift,
            ),
            buchung(
                person("PERSON_1"),
                123_450,
                Richtung::Ausgang,
                Gegenkonto::Fremd,
                Some("eine-nachkommastelle"),
                Buchungsart::Lastschrift,
            ),
        ]
    );
}

#[test]
fn punkt_als_dezimaltrenner_ist_ein_fehler() {
    // `1234.56` (Punkt statt Komma) würde sonst still um Faktor 100 falsch
    // verrechnet — muss laut abbrechen.
    let result = import_inline(&[Row {
        betrag: "1234.56",
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::BetragInvalid(_))));
}

#[test]
fn falsch_gruppierte_tausenderpunkte_sind_ein_fehler() {
    let result = import_inline(&[Row {
        betrag: "1..2,00",
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::BetragInvalid(_))));
}

#[test]
fn fehlermeldung_enthält_keine_rohe_eingabe() {
    // Ein ungültiges Auftragskonto darf im Klartext nicht in der Fehlermeldung
    // stehen — sonst leakt ein (fast-)struktureller Identifikator über Logs.
    let raw = "DE00 1234 5678";
    let error = import_inline(&[Row {
        auftragskonto: raw,
        ..Row::standard()
    }])
    .expect_err("ungültiges Auftragskonto");

    let message = error.to_string();
    assert!(
        !message.contains(raw) && !message.contains("DE0012345678"),
        "rohe Eingabe in Fehlermeldung geleakt: {message}"
    );
}

#[test]
fn csv_quoting_mit_eingebettetem_semikolon_bleibt_ein_feld() {
    // Ein gequoteter Verwendungszweck mit eingebettetem Trennzeichen bleibt ein
    // Feld (die csv-Crate entquotet korrekt).
    let content = format!(
        "{HEADER}\n\"{konto}\";\"01.01.2026\";\"01.01.2026\";\"LASTSCHRIFT\";\
\"Rechnung 1; Teil 2\";\"DE98ZZZ09999999999\";\"\";\"\";\"\";\"\";\"\";\
\"Firma AG\";\"{fremd}\";\"\";\"-3,00\";\"EUR\";\"\"",
        konto = EIGENE_GIRO,
        fremd = FREMDE_IBAN,
    );
    let buchungen = load(&CsvCamtV8::new(StringSource(content), konten())).expect("importiert");

    assert_eq!(
        buchungen,
        vec![buchung(
            firma("DE98ZZZ09999999999"),
            300,
            Richtung::Ausgang,
            Gegenkonto::Fremd,
            Some("Rechnung 1; Teil 2"),
            Buchungsart::Lastschrift,
        )]
    );
}

#[test]
fn komma_ohne_nachkommastellen_ist_ein_fehler() {
    // `12,` ist eine strukturell kaputte Zelle — lautes Scheitern statt raten.
    let result = import_inline(&[Row {
        betrag: "12,",
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::BetragInvalid(_))));
}

#[test]
fn führende_null_in_tausendergruppe_ist_ein_fehler() {
    // `01.234` ist kein valider Tausendertrenner-Ausdruck.
    let result = import_inline(&[Row {
        betrag: "01.234,56",
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::BetragInvalid(_))));
}

#[test]
fn fehlermeldung_zu_fremdem_auftragskonto_leakt_die_iban_nicht() {
    // Auftragskonto ist eine gültige Fremd-IBAN (nicht in der Config) → Fehler;
    // die Meldung darf die IBAN nicht im Klartext tragen.
    let error = import_inline(&[Row {
        auftragskonto: FREMDE_IBAN,
        ..Row::standard()
    }])
    .expect_err("Auftragskonto nicht in Config");

    let message = error.to_string();
    assert!(
        !message.contains(FREMDE_IBAN) && !message.contains("DE90876543210000009999"),
        "IBAN in Fehlermeldung geleakt: {message}"
    );
}

#[test]
fn führende_null_ohne_gruppierung_ist_ein_fehler() {
    // `007,50` ist kein valider Betrag — führende Nullen werden konsistent
    // abgelehnt (die echte `0` bleibt zulässig).
    let result = import_inline(&[Row {
        betrag: "007,50",
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::BetragInvalid(_))));
}

#[test]
fn nur_vorzeichen_ohne_ziffern_ist_ein_fehler() {
    let result = import_inline(&[Row {
        betrag: "-",
        ..Row::standard()
    }]);

    assert!(matches!(result, Err(ImportError::BetragInvalid(_))));
}
