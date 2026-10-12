//! Der **Vier-Konten-Nachweis** ([Abschlussplan](../../../../docs/plans/2026-10-12-1343-abschlussplan.md),
//! Definition „Projekt fertig", Punkte **1 und 4**): je eigenem Konto ein Lauf — Giro, Haushalt,
//! Spar, Vorlauf —, alle in **eine** echte SQLite-Datei, über die echten Adapter, ohne einen Fake.
//!
//! - **Punkt 1:** nach dem vierten Lauf zeigt der Bericht alle Konten gemeinsam, und dieselben vier
//!   Läufe ein zweites Mal ändern keine Zahl.
//! - **Punkt 4:** weder Bericht noch Chat-Export tragen eine IBAN, eine BIC, eine Kontonummer, eine
//!   Referenz, einen Klarnamen (auch kein Wort davon) oder den Seed — auch nicht aus Verwendungszwecken,
//!   die eine IBAN, eine BIC, eine Kontonummer oder den Namen eines bekannten Empfängers tragen.
//!
//! Dass jede Berichts-Sektion über vier Konten gefüllt ist, prüft der erste Test mit; die Regeln der
//! Sektionen selbst (Punkt 2) belegen `bericht_tests`, Export, Import und Korrektur (Punkt 3)
//! `kategorien_tests`. Die Daten sind erfunden; ihre Konvention steht in `fixtures/e2e/README.md`.

use std::collections::BTreeSet;

use super::*;
use crate::pipeline_test_support::{TempDbPath, VerboteneWerte};

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

/// Die Pfade des Laufs über `auszug`; nur mit `export` schreibt er den Chat-Export.
fn pfade(db: &TempDbPath, auszug: &str, export: bool) -> RunPaths {
    RunPaths {
        database: db.path().to_path_buf(),
        transactions: fixture(auszug),
        window_config: fixture("fenster.toml"),
        seed_config: fixture("seed.toml"),
        own_accounts_config: fixture("konten.toml"),
        persons_config: fixture("namen.toml"),
        savings_config: fixture("sparen.toml"),
        category_export: export.then(|| db.export_path()),
        report: db.report_path(),
    }
}

/// Fährt die vier Läufe nacheinander in dieselbe Datei; nur der **letzte** schreibt den Export. Jeder
/// Lauf muss gelingen (Exit 0) und sieht jede eingelesene Buchung.
fn vier_laeufe(db: &TempDbPath) -> Vec<RunReport> {
    AUSZUEGE
        .iter()
        .enumerate()
        .map(|(index, auszug)| {
            let letzter = index + 1 == AUSZUEGE.len();
            let bilanz = run(&pfade(db, auszug, letzter), &mut Vec::new())
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

fn export(db: &TempDbPath) -> String {
    std::fs::read_to_string(db.export_path()).expect("der Export ist geschrieben")
}

/// Eine Tabelle des Berichts: die Spaltenüberschriften und je Zeile die Zellen-Texte (noch
/// HTML-escaped). Zellen werden über den **Spaltennamen** gelesen, nicht über ihre Stelle.
#[derive(Debug, PartialEq, Eq)]
struct Tabelle {
    spalten: Vec<String>,
    zeilen: Vec<Vec<String>>,
}

impl Tabelle {
    /// Der Abschnitt `id` des Berichts — ohne Zeilen eine leere Tabelle.
    fn aus(html: &str, id: &str) -> Self {
        let anfang = format!("<section id=\"{id}\">");
        let inhalt = html
            .split_once(anfang.as_str())
            .and_then(|(_, rest)| rest.split_once("</section>"))
            .map(|(inhalt, _)| inhalt)
            .ok_or_else(|| format!("Abschnitt {id}"))
            .expect("der Bericht trägt den Abschnitt");
        let spalten = inhalt
            .split_once("<thead><tr>")
            .and_then(|(_, kopf)| kopf.split_once("</tr>"))
            .map_or_else(Vec::new, |(kopf, _)| zellen(kopf, "th"));
        let zeilen = inhalt
            .split("<tr>")
            .filter(|zeile| zeile.contains("<td"))
            .map(|zeile| zellen(zeile, "td"))
            .collect();
        Self { spalten, zeilen }
    }

    fn index(&self, spalte: &str) -> usize {
        self.spalten
            .iter()
            .position(|titel| titel == spalte)
            .ok_or_else(|| format!("Spalte {spalte} in {:?}", self.spalten))
            .expect("die Tabelle hat die Spalte")
    }

    /// Jede Zeile, beschränkt auf die genannten Spalten in dieser Reihenfolge.
    fn projiziert(&self, spalten: &[&str]) -> Vec<Vec<String>> {
        let indizes = spalten
            .iter()
            .map(|spalte| self.index(spalte))
            .collect::<Vec<_>>();
        self.zeilen
            .iter()
            .map(|zeile| {
                indizes
                    .iter()
                    .map(|&index| zeile.get(index).cloned().unwrap_or_default())
                    .collect()
            })
            .collect()
    }

    /// Die Zelle `spalte` der Zeile, deren Zelle `schluessel` gleich `wert` ist.
    fn zelle(&self, schluessel: &str, wert: &str, spalte: &str) -> String {
        let (schluessel, spalte) = (self.index(schluessel), self.index(spalte));
        self.zeilen
            .iter()
            .find(|zeile| zeile.get(schluessel).is_some_and(|zelle| zelle == wert))
            .and_then(|zeile| zeile.get(spalte).cloned())
            .ok_or_else(|| format!("keine Zeile für {wert}: {:?}", self.zeilen))
            .expect("die Tabelle trägt die Zeile")
    }
}

/// Die Texte der `<tag …>…</tag>`-Zellen eines Zeilen-Ausschnitts.
fn zellen(ausschnitt: &str, tag: &str) -> Vec<String> {
    let (auf, zu) = (format!("<{tag}"), format!("</{tag}>"));
    ausschnitt
        .split(auf.as_str())
        .skip(1)
        .filter_map(|zelle| {
            let (_, text) = zelle.split_once('>')?;
            let (text, _) = text.split_once(zu.as_str())?;
            Some(text.to_string())
        })
        .collect()
}

/// Alle sechs Abschnitte — die **Zahlen** des Berichts, ohne Kopf und Fuß.
fn abschnitte(html: &str) -> Vec<Tabelle> {
    ABSCHNITTE.iter().map(|id| Tabelle::aus(html, id)).collect()
}

fn zeile(zellen: &[&str]) -> Vec<String> {
    zellen.iter().map(ToString::to_string).collect()
}

#[test]
fn vier_konten_in_einer_datei_ergeben_einen_bericht_ueber_alle_konten() {
    let db = TempDbPath::new("e2e_vier_konten");

    vier_laeufe(&db);
    let html = bericht(&db);

    // Cashflow: alle vier Rollen, und Umbuchungen sind herausgerechnet. Giro Februar: Gehalt hinein;
    // hinaus Miete 950,00 + Abo 12,99 + Musikdienst 9,99 + Cloudspeicher 2,99 + Entgelt 4,90 +
    // Rückzahlung 60,00 — die Daueraufträge auf Haushalt (400), Vorlauf (150) und Spar (200) nicht.
    let cashflow = Tabelle::aus(&html, "cashflow");
    let cashflow_zeilen = cashflow.projiziert(&["Konto", "Monat", "Eingang", "Ausgang"]);
    let konten = cashflow
        .projiziert(&["Konto"])
        .into_iter()
        .flatten()
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
        assert!(cashflow_zeilen.contains(&zeile(&erwartet)), "{erwartet:?}");
    }

    // Umbuchungen: dort steht, was aus dem Cashflow herausfiel — je Monat und Gegenkonto.
    let umbuchungen = Tabelle::aus(&html, "umbuchungen").projiziert(&[
        "Monat",
        "Gegenkonto",
        "Eingang",
        "Ausgang",
    ]);
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
    let gebuehren =
        Tabelle::aus(&html, "gebuehren").projiziert(&["Monat", "Konto", "Eingezogen", "Erstattet"]);
    for erwartet in [
        ["Januar 2026", "Giro", "4,90 €", "0,00 €"],
        ["März 2026", "Haushalt", "7,50 €", "0,00 €"],
    ] {
        assert!(gebuehren.contains(&zeile(&erwartet)), "{erwartet:?}");
    }

    // Wiederkehr: monatlich mit Preissteigerung, jährlich, und zwei Händler hinter einem Durchleiter
    // als zwei Zeilen.
    let wiederkehr = Tabelle::aus(&html, "wiederkehr");
    assert_eq!(
        wiederkehr.zelle("Treiber", "STREAMHAUS FIKTIV", "Takt"),
        "monatlich"
    );
    assert_eq!(
        wiederkehr.zelle("Treiber", "STREAMHAUS FIKTIV", "Preissteigerung"),
        "seit 15.04.2026: 12,99 € → 14,99 €"
    );
    let versicherung = "HAUSRAT FIKTIV VERSICHERUNG AG";
    assert_eq!(
        wiederkehr.zelle("Treiber", versicherung, "Takt"),
        "jährlich"
    );
    assert_eq!(
        wiederkehr.zelle("Treiber", versicherung, "Konto"),
        "Vorlauf"
    );
    for haendler in ["MUSIKDIENST", "CLOUDSPEICHER"] {
        let treiber = format!("PAYPAL FIKTIV ZAHLDIENST › {haendler}-FIKTIV-IHR-EINKAUF");
        assert_eq!(wiederkehr.zelle("Treiber", &treiber, "Takt"), "monatlich");
    }

    // Ranking und Spar-Kandidaten: gefüllt, die Anteile summieren sich zu 100 %.
    let kumuliert = Tabelle::aus(&html, "ranking").projiziert(&["Kumuliert"]);
    assert!(kumuliert.len() > 5, "{kumuliert:?}");
    assert_eq!(kumuliert.last(), Some(&zeile(&["100,00 %"])));
    assert_eq!(
        Tabelle::aus(&html, "spar-kandidaten").zelle(
            "Treiber",
            "STREAMHAUS FIKTIV",
            "Jahresbetrag"
        ),
        "179,88 €"
    );

    // Der Chat-Export trägt die unbekannten Firmen und Händler — die Praxis, deren Name eine Person
    // der Namensliste nennt, nur mit gescrubbtem Namen.
    let inhalt = export(&db);
    for erwartet in [
        "COMPANY:DE37ZZZFIKTIV00001",
        "MERCHANT:MUSIKDIENST-FIKTIV-IHR-EINKAUF",
        "PRAXIS [PERSON]",
    ] {
        assert!(inhalt.contains(erwartet), "{erwartet}");
    }
}

#[test]
fn vier_konten_ein_zweites_mal_ergeben_dieselben_zahlen() {
    let db = TempDbPath::new("e2e_wiederholung");

    let erste_runde = vier_laeufe(&db);
    let erster_bericht = bericht(&db);
    // Bericht und Export der ersten Runde weg — die zweite Runde muss beide neu schreiben.
    for datei in [db.report_path(), db.export_path()] {
        std::fs::remove_file(&datei).expect("die Datei der ersten Runde ist da");
    }
    let zweite_runde = vier_laeufe(&db);
    assert!(db.report_path().exists() && db.export_path().exists());
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
        erste.iter().all(|tabelle| !tabelle.zeilen.is_empty()),
        "{erste:?}"
    );
    assert_eq!(erste, abschnitte(&zweiter_bericht));
}

#[test]
fn bericht_und_export_von_vier_konten_tragen_keine_iban_keine_bic_keinen_klarnamen() {
    let db = TempDbPath::new("e2e_leak");

    vier_laeufe(&db);
    let (html, inhalt) = (bericht(&db), export(&db));

    let verboten = AUSZUEGE
        .iter()
        .map(|auszug| VerboteneWerte::aus_lauf(&pfade(&db, auszug, false)))
        .fold(VerboteneWerte::default(), VerboteneWerte::und);
    // Die Liste prüft, was die Fixtures anbieten: die eigenen IBANs (auch in Leerzeichen-Schreibweise)
    // samt Kontonummer, ein Gegenkonto, eine BIC, eine Referenz, die Person, die Bäckerei und die
    // Gärtnerei ohne Gläubiger-ID (ganz und Wort für Wort, mit Umlaut und `&`) und den Namen aus der
    // Namensliste.
    for muss in [
        "DE27 1000 0000 0000 0041 01",
        "DE27100000000000004101",
        "4101",
        "57381946",
        "DE00200000000000000202",
        "FIKTDEFF707",
        "MND-PP-0001",
        "E2E-GI-2603-10",
        "ERWIN BEISPIELMANN",
        "BEISPIELMANN",
        "BAECKEREI FIKTIV",
        "GÄRTNEREI GRÜN & SÖHNE",
        "SÖHNE",
        "Tilda Erfunden",
        "ERFUNDEN",
    ] {
        assert!(verboten.enthaelt(muss), "{muss} fehlt in {verboten:?}");
    }

    // Die drei Verwendungszwecke, die das Scrubbing tragen muss, gehen über den Durchleiter in den
    // Export: eine IBAN mit Leerzeichen und eine BIC, der volle Name eines bekannten Empfängers und
    // eine reine Kontonummer.
    verboten.pruefe("Bericht", &html);
    verboten.pruefe("Export", &inhalt);
    // … und sie kommen gescrubbt an, statt ganz wegzufallen (sonst bliebe die Prüfung oben grün,
    // ohne etwas zu belegen).
    for gescrubbt in [
        "\". Erstattung an [IBAN] BIC [BIC]\"",
        "\". Geschenk fuer [PERSON]\"",
        "\". Rueckgabe Konto\"",
    ] {
        assert!(inhalt.contains(gescrubbt), "der Export trägt {gescrubbt}");
    }
    assert!(
        html.contains("PAYPAL FIKTIV ZAHLDIENST › GESCHENK-FUER-PERSON"),
        "der Bericht trägt den gescrubbten Treiber"
    );
    assert!(!inhalt.contains("PERSON:"), "keine Person geht hinaus");
}
