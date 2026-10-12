//! Der **Vier-Konten-Nachweis** ([Abschlussplan](../../../../docs/plans/2026-10-12-1343-abschlussplan.md),
//! Definition „Projekt fertig", Punkte 1–4): je eigenem Konto ein Lauf — Giro, Haushalt, Spar,
//! Vorlauf —, alle in **eine** echte SQLite-Datei, über die echten Adapter, ohne einen Fake. Nach dem
//! vierten Lauf zeigt der Bericht alle Konten gemeinsam, der Chat-Export trägt die unbekannten
//! Buchungsarten, dieselben vier Läufe ein zweites Mal ändern keine Zahl, und weder Bericht noch
//! Export tragen eine IBAN, eine BIC, einen Klarnamen oder den Seed.
//!
//! Die Daten sind erfunden; ihre Konvention steht in `fixtures/e2e/README.md`.

use std::collections::BTreeSet;

use super::bericht_tests::rohwerte;
use super::*;
use crate::pipeline_test_support::TempDbPath;

/// Die vier Auszüge in der Reihenfolge der Läufe — je eigenem Konto einer.
const AUSZUEGE: [&str; 4] = ["giro.csv", "haushalt.csv", "spar.csv", "vorlauf.csv"];

/// Die sechs Abschnitte des Berichts (ihre Anker im HTML).
const ABSCHNITTE: [&str; 6] = [
    "cashflow",
    "umbuchungen",
    "gebuehren",
    "ranking",
    "spar-kandidaten",
    "wiederkehr",
];

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/fixtures/e2e/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

/// Die Export-Datei neben der Datenbank — derselbe eindeutige Stamm; verschwindet nach dem Test.
struct ExportDatei(PathBuf);

impl ExportDatei {
    fn neben(db: &TempDbPath) -> Self {
        Self(db.path().with_extension("json"))
    }
}

impl Drop for ExportDatei {
    fn drop(&mut self) {
        // Best-effort-Aufräumen: eine fehlende Datei ist kein Fehler.
        match std::fs::remove_file(&self.0) {
            Ok(()) | Err(_) => {}
        }
    }
}

fn pfade(db: &TempDbPath, auszug: &str, export: Option<&ExportDatei>) -> RunPaths {
    RunPaths {
        database: db.path().to_path_buf(),
        transactions: fixture(auszug),
        window_config: fixture("fenster.toml"),
        seed_config: fixture("seed.toml"),
        own_accounts_config: fixture("konten.toml"),
        persons_config: fixture("namen.toml"),
        savings_config: fixture("sparen.toml"),
        category_export: export.map(|datei| datei.0.clone()),
        report: db.report_path(),
    }
}

/// Fährt die vier Läufe nacheinander in dieselbe Datei; nur der **letzte** schreibt den Export. Jeder
/// Lauf muss gelingen (Exit 0) und sieht jede eingelesene Buchung.
fn vier_laeufe(db: &TempDbPath, export: &ExportDatei) -> Vec<RunReport> {
    AUSZUEGE
        .iter()
        .enumerate()
        .map(|(index, auszug)| {
            let letzter = index + 1 == AUSZUEGE.len();
            let bilanz = run(
                &pfade(db, auszug, letzter.then_some(export)),
                &mut Vec::new(),
            )
            .map_err(|fehler| format!("{auszug}: {fehler}"))
            .expect("jeder der vier Läufe gelingt (Exit 0)");
            assert_eq!(bilanz.incomplete(), 0, "{auszug}: keine Buchung fehlt");
            bilanz
        })
        .collect()
}

fn bericht(db: &TempDbPath) -> String {
    std::fs::read_to_string(db.report_path()).expect("der Bericht ist geschrieben")
}

/// Die Zeilen eines Abschnitts als Zellen-Texte (noch HTML-escaped) — `None`, wenn der Abschnitt
/// fehlt. Ein Abschnitt ohne Zeilen ergibt eine leere Liste.
fn abschnitt(html: &str, id: &str) -> Option<Vec<Vec<String>>> {
    let anfang = format!("<section id=\"{id}\">");
    let (_, rest) = html.split_once(anfang.as_str())?;
    let (inhalt, _) = rest.split_once("</section>")?;
    Some(
        inhalt
            .split("<tr>")
            .filter(|zeile| zeile.contains("<td"))
            .map(|zeile| {
                zeile
                    .split("<td")
                    .skip(1)
                    .filter_map(|zelle| {
                        let (_, text) = zelle.split_once('>')?;
                        let (text, _) = text.split_once("</td>")?;
                        Some(text.to_string())
                    })
                    .collect()
            })
            .collect(),
    )
}

/// Alle sechs Abschnitte — die **Zahlen** des Berichts, ohne Kopf und Fuß.
fn abschnitte(html: &str) -> Vec<(&'static str, Vec<Vec<String>>)> {
    ABSCHNITTE
        .iter()
        .map(|&id| {
            let zeilen = abschnitt(html, id)
                .ok_or_else(|| format!("Abschnitt {id}"))
                .expect("der Bericht trägt jeden Abschnitt");
            (id, zeilen)
        })
        .collect()
}

fn zeile(zellen: &[&str]) -> Vec<String> {
    zellen.iter().map(ToString::to_string).collect()
}

/// Die Zeile eines Abschnitts, deren **erste** Zelle `treiber` ist.
fn zeile_von<'z>(zeilen: &'z [Vec<String>], treiber: &str) -> &'z [String] {
    zeilen
        .iter()
        .find(|zeile| zeile.first().is_some_and(|zelle| zelle == treiber))
        .ok_or_else(|| format!("keine Zeile für {treiber}: {zeilen:?}"))
        .expect("der Abschnitt trägt den Treiber")
}

/// Großgeschrieben und nur noch Buchstaben und Ziffern — so fällt auch eine IBAN auf, die ohne
/// Leerzeichen, mit Punkten oder klein geschrieben dasteht.
fn normalisiert(text: &str) -> String {
    text.chars()
        .filter(|zeichen| zeichen.is_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

/// Die IBANs der Eigene-Konten-Config, so wie sie in der Datei stehen (auch mit Leerzeichen).
fn eigene_ibans() -> Vec<String> {
    std::fs::read_to_string(fixture("konten.toml"))
        .expect("die Konten-Config ist lesbar")
        .lines()
        .filter_map(|zeile| zeile.trim().strip_prefix("iban"))
        .filter_map(|rest| rest.split('"').nth(1))
        .map(ToString::to_string)
        .collect()
}

#[test]
fn vier_konten_in_einer_datei_ergeben_einen_bericht_ueber_alle_konten() {
    let db = TempDbPath::new("e2e_vier_konten");
    let export = ExportDatei::neben(&db);

    vier_laeufe(&db, &export);
    let html = bericht(&db);

    // Cashflow: alle vier Rollen, und Umbuchungen sind herausgerechnet. Giro Februar: Gehalt hinein;
    // hinaus Miete 950,00 + Abo 12,99 + Musikdienst 9,99 + Cloudspeicher 2,99 + Entgelt 4,90 +
    // Rückzahlung 60,00 — die Daueraufträge auf Haushalt (400), Vorlauf (150) und Spar (200) nicht.
    let cashflow = abschnitt(&html, "cashflow").expect("ein Cashflow-Abschnitt");
    let konten = cashflow
        .iter()
        .filter_map(|zeile| zeile.first().cloned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        konten,
        ["Giro", "Haushalt", "Spar", "Vorlauf"]
            .map(String::from)
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
    for erwartet in [
        ["Giro", "Februar 2026", "2.800,00 €", "1.040,87 €"],
        // Mai: die Entnahme vom Sparkonto (500,00) ist kein Eingang.
        ["Giro", "Mai 2026", "2.800,00 €", "982,87 €"],
        // Juni: Strom 78,00 + Markt 59,31 + 70,02 + Bäckerei 7,10 + Entgelt 7,50 — der vorgemerkte
        // Umsatz (12,35) zählt nicht, das Haushaltsgeld vom Giro (400,00) ist kein Eingang.
        ["Haushalt", "Juni 2026", "0,00 €", "221,93 €"],
        ["Spar", "Juni 2026", "3,12 €", "0,00 €"],
        ["Vorlauf", "März 2025", "0,00 €", "186,40 €"],
    ] {
        assert!(cashflow.contains(&zeile(&erwartet)), "{erwartet:?}");
    }

    // Umbuchungen: dort steht, was aus dem Cashflow herausfiel — je Monat und Gegenkonto.
    let umbuchungen = abschnitt(&html, "umbuchungen").expect("ein Umbuchungs-Abschnitt");
    for erwartet in [
        ["Februar 2026", "Haushalt", "0,00 €", "400,00 €"],
        ["Februar 2026", "Spar", "0,00 €", "200,00 €"],
        ["Februar 2026", "Vorlauf", "0,00 €", "150,00 €"],
        ["Februar 2026", "Giro", "750,00 €", "0,00 €"],
        ["Mai 2026", "Spar", "500,00 €", "200,00 €"],
        ["Mai 2026", "Giro", "750,00 €", "500,00 €"],
    ] {
        assert!(umbuchungen.contains(&zeile(&erwartet)), "{erwartet:?}");
    }

    // Gebühren: monatlich auf Giro, quartalsweise auf Haushalt.
    let gebuehren = abschnitt(&html, "gebuehren").expect("ein Gebühren-Abschnitt");
    for erwartet in [
        ["Januar 2026", "Giro", "4,90 €", "0,00 €"],
        ["März 2026", "Haushalt", "7,50 €", "0,00 €"],
    ] {
        assert!(gebuehren.contains(&zeile(&erwartet)), "{erwartet:?}");
    }

    // Wiederkehr: monatlich mit Preissteigerung, jährlich, und zwei Händler hinter einem Durchleiter
    // als zwei Zeilen.
    let wiederkehr = abschnitt(&html, "wiederkehr").expect("ein Wiederkehr-Abschnitt");
    let abo = zeile_von(&wiederkehr, "STREAMHAUS FIKTIV");
    assert_eq!(abo.get(2).map(String::as_str), Some("monatlich"));
    assert_eq!(
        abo.get(5).map(String::as_str),
        Some("seit 15.04.2026: 12,99 € → 14,99 €")
    );
    let versicherung = zeile_von(&wiederkehr, "HAUSRAT FIKTIV VERSICHERUNG AG");
    assert_eq!(versicherung.get(2).map(String::as_str), Some("jährlich"));
    assert_eq!(versicherung.get(6).map(String::as_str), Some("Vorlauf"));
    for haendler in ["MUSIKDIENST", "CLOUDSPEICHER"] {
        let treiber = format!("PAYPAL FIKTIV ZAHLDIENST › {haendler}-FIKTIV-IHR-EINKAUF");
        assert_eq!(
            zeile_von(&wiederkehr, &treiber).get(2).map(String::as_str),
            Some("monatlich")
        );
    }

    // Ranking und Spar-Kandidaten: gefüllt, die Anteile summieren sich zu 100 %.
    let ranking = abschnitt(&html, "ranking").expect("ein Ranking-Abschnitt");
    assert!(ranking.len() > 5, "{ranking:?}");
    assert_eq!(
        ranking
            .last()
            .and_then(|zeile| zeile.get(6))
            .map(String::as_str),
        Some("100,00 %")
    );
    let kandidaten = abschnitt(&html, "spar-kandidaten").expect("ein Spar-Kandidaten-Abschnitt");
    assert_eq!(
        zeile_von(&kandidaten, "STREAMHAUS FIKTIV")
            .get(2)
            .map(String::as_str),
        Some("179,88 €")
    );

    // Der Chat-Export: nur Firmen und Händler, keine Person — die Praxis über ihren gescrubbten Namen.
    let inhalt = std::fs::read_to_string(&export.0).expect("der Export ist geschrieben");
    for erwartet in [
        "COMPANY:DE37ZZZFIKTIV00001",
        "MERCHANT:MUSIKDIENST-FIKTIV-IHR-EINKAUF",
        "PRAXIS [PERSON]",
    ] {
        assert!(inhalt.contains(erwartet), "{erwartet}");
    }
    assert!(!inhalt.contains("PERSON:"), "keine Person geht hinaus");
}

#[test]
fn vier_konten_ein_zweites_mal_ergeben_dieselben_zahlen() {
    let db = TempDbPath::new("e2e_wiederholung");
    let export = ExportDatei::neben(&db);

    let erste_runde = vier_laeufe(&db, &export);
    let erster_bericht = bericht(&db);
    let zweite_runde = vier_laeufe(&db, &export);
    let zweiter_bericht = bericht(&db);

    // Jeder Lauf mintet seine eigene Identität, und die zweite Runde liest nichts neu ein.
    let ids = erste_runde
        .iter()
        .chain(&zweite_runde)
        .map(RunReport::run_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 8);
    for bilanz in &zweite_runde {
        assert_eq!(
            bilanz.processed().get(OBFUSKIERUNG_CONSUMED),
            None,
            "eine bekannte Zeile wird nicht erneut publiziert"
        );
    }
    let letzte = |runde: &[RunReport]| {
        runde
            .last()
            .map(|bilanz| (bilanz.occurrences(), bilanz.savings_candidates()))
    };
    assert_eq!(letzte(&erste_runde), letzte(&zweite_runde));

    // Dieselben Zahlen in jedem Abschnitt.
    let erste = abschnitte(&erster_bericht);
    assert!(
        erste.iter().all(|(_, zeilen)| !zeilen.is_empty()),
        "{erste:?}"
    );
    assert_eq!(erste, abschnitte(&zweiter_bericht));
}

#[test]
fn bericht_und_export_von_vier_konten_tragen_keine_iban_keine_bic_keinen_klarnamen() {
    let db = TempDbPath::new("e2e_leak");
    let export = ExportDatei::neben(&db);

    vier_laeufe(&db, &export);
    let ausgaben = [
        ("Bericht", bericht(&db)),
        (
            "Export",
            std::fs::read_to_string(&export.0).expect("der Export ist geschrieben"),
        ),
    ];

    // Aus den Fixtures selbst: jede IBAN (Auftragskonto, Gegenkonto, Eigene-Konten-Config), jede BIC,
    // jeder Name einer Gegenpartei ohne Gläubiger-ID (einer fail-safe Person), jeder Name der
    // Namensliste und der Seed. Firmennamen mit Gläubiger-ID sind öffentlicher Analyse-Anker und
    // bleiben erlaubt.
    let mut verboten = AUSZUEGE
        .iter()
        .flat_map(|auszug| rohwerte(&fixture(auszug)).expect("der Auszug ist lesbar"))
        .collect::<BTreeSet<_>>();
    verboten.extend(eigene_ibans());
    verboten.extend(
        PersonenConfig::load(&fixture("namen.toml"))
            .expect("die Namensliste lädt")
            .into_names(),
    );
    verboten.insert(
        SeedConfig::load(&fixture("seed.toml"))
            .expect("die Seed-Config lädt")
            .seed()
            .to_string(),
    );
    // Die Liste prüft etwas: die vier eigenen IBANs (auch in Leerzeichen-Schreibweise), Gegenkonten,
    // BICs, die Person, die Bäckerei ohne Gläubiger-ID und der Name aus der Namensliste.
    for muss in [
        "DE27 1000 0000 0000 0041 01",
        "DE27100000000000004101",
        "DE00200000000000000202",
        "FIKTDEFF202",
        "ERWIN BEISPIELMANN",
        "BAECKEREI FIKTIV",
        "Tilda Erfunden",
    ] {
        assert!(verboten.contains(muss), "{muss} fehlt in {verboten:?}");
    }

    for (name, text) in &ausgaben {
        // Ein Konto-Token ist kein Rohwert, aber auch er bleibt drinnen (nur roh geprüft: normalisiert
        // träfe „KONTO" schon jede Spaltenüberschrift).
        assert!(!text.contains("KONTO_"), "{name} trägt ein Konto-Token");
        let text_normalisiert = normalisiert(text);
        for wert in &verboten {
            assert!(!text.contains(wert.as_str()), "{name} trägt {wert}");
            assert!(
                !text_normalisiert.contains(&normalisiert(wert)),
                "{name} trägt {wert} (normalisiert)"
            );
        }
    }
}
