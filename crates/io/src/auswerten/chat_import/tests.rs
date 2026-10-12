//! Der Import einer Chat-Antwort gegen **echtes** SQLite (Issue 0011): nur über den exakten
//! Fingerabdruck, nie über Text; ein bekanntes Etikett bleibt ohne Korrektur-Schalter; Widerspruch,
//! kaputtes JSON und eine zu große Datei weisen alles ab — und keine Meldung nennt einen Wert.

use super::*;

use crate::config::AppConfig;
use crate::pipeline_test_support::TempDbPath;
use crate::schema;

/// Erfundene, von einer (erfundenen) Auswertung ausgegebene Fingerabdrücke.
const STROM: &str = "COMPANY:DE98ZZZ09999999999";
const VERSICHERUNG: &str = "COMPANY:DE13ZZZ00000000123";
const MUSIK: &str = "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:MUSIKDIENST-BEISPIEL";

/// Eine Antwort-Datei im Temp-Verzeichnis, die nach dem Test verschwindet.
struct AntwortDatei(std::path::PathBuf);

impl AntwortDatei {
    fn mit(tag: &str, inhalt: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|dauer| dauer.as_nanos())
            .unwrap_or(0);
        let pfad = std::env::temp_dir().join(format!(
            "bank_auswertung_antwort_{tag}_{}_{nanos}.json",
            std::process::id()
        ));
        std::fs::write(&pfad, inhalt).expect("die Antwort-Datei lässt sich schreiben");
        Self(pfad)
    }
}

impl Drop for AntwortDatei {
    fn drop(&mut self) {
        let _aufraeumen: std::io::Result<()> = std::fs::remove_file(&self.0);
    }
}

/// Öffnet die Datei wie ein Programmstart und lässt eine erfundene Auswertung die drei
/// Fingerabdrücke ausgeben.
fn register(db: &TempDbPath) -> SqliteCategoryRegistry {
    let outbox =
        schema::open(&AppConfig::new(db.path())).expect("die Test-Datei lässt sich öffnen");
    outbox
        .with_connection(|conn| {
            [STROM, VERSICHERUNG, MUSIK]
                .iter()
                .zip(0_i64..)
                .try_for_each(|(fingerprint, position)| {
                    conn.execute(
                        "INSERT OR IGNORE INTO window_evaluation_fingerprints \
                         (evaluation_id, position, fingerprint) VALUES ('auswertung-1', ?1, ?2);",
                        (position, fingerprint),
                    )
                    .map(drop)
                })
        })
        .expect("die Auswertung führt die Fingerabdrücke");
    SqliteCategoryRegistry::new(outbox)
}

fn etiketten(registry: &SqliteCategoryRegistry) -> Vec<(String, String)> {
    registry
        .categories_of(&[STROM, VERSICHERUNG, MUSIK].map(ToString::to_string))
        .expect("das Register ist lesbar")
        .into_iter()
        .map(|answer| (answer.fingerprint, answer.category))
        .collect()
}

fn paar(fingerprint: &str, category: &str) -> (String, String) {
    (fingerprint.to_string(), category.to_string())
}

#[test]
fn gelernt_wird_nur_ueber_den_exakten_fingerabdruck_nie_ueber_text() {
    let db = TempDbPath::new("import_exakt");
    let registry = register(&db);
    let antwort = AntwortDatei::mit(
        "exakt",
        &format!(
            r#"{{"antworten": [
                {{"fingerabdruck": "STADTWERKE BEISPIELSTADT", "text": "Stadtwerke Beispielstadt", "kategorie": "Wohnen"}},
                {{"fingerabdruck": "{lower}", "kategorie": "Versicherung"}},
                {{"text": "Musikdienst Beispiel", "kategorie": "Unterhaltung"}},
                {{"fingerabdruck": "{MUSIK}", "kategorie": "Unterhaltung", "begruendung": "Musik-Abo"}}
            ]}}"#,
            lower = VERSICHERUNG.to_lowercase()
        ),
    );

    let bericht = import(&registry, &antwort.0, ImportMode::LearnUnknown).expect("gelesen");

    // Falscher Schlüssel + passender Text → verworfen; andere Groß-/Kleinschreibung → verworfen; Text
    // ohne Fingerabdruck → verworfen. Richtiger Fingerabdruck + Zusatzfeld → gelernt, das Zusatzfeld
    // nie gelesen, nur gezählt (dazu zählt auch das `text` des ersten Eintrags).
    assert_eq!(
        bericht,
        ImportReport {
            learned: 1,
            kept_known: vec![],
            discarded: vec![
                (1, DiscardCause::NotIssued),
                (2, DiscardCause::NotIssued),
                (3, DiscardCause::MissingFingerprint),
            ],
            ignored_fields: 2,
        }
    );
    assert_eq!(etiketten(&registry), vec![paar(MUSIK, "Unterhaltung")]);
}

#[test]
fn ein_bekanntes_etikett_bleibt_ohne_korrektur_und_wird_mit_korrektur_ersetzt() {
    let db = TempDbPath::new("import_korrektur");
    let registry = register(&db);
    registry
        .learn(&[CategoryAnswerRow {
            fingerprint: STROM.to_string(),
            category: "Wohnen & Energie".to_string(),
        }])
        .expect("ein schon gelerntes Etikett");
    let antwort = AntwortDatei::mit(
        "korrektur",
        &format!(r#"{{"antworten": [{{"fingerabdruck": "{STROM}", "kategorie": "Sonstiges"}}]}}"#),
    );

    let ohne = import(&registry, &antwort.0, ImportMode::LearnUnknown).expect("gelesen");
    assert_eq!(ohne.learned, 0);
    assert_eq!(ohne.kept_known, vec![1]);
    assert_eq!(etiketten(&registry), vec![paar(STROM, "Wohnen & Energie")]);

    let mit = import(&registry, &antwort.0, ImportMode::Correct).expect("gelesen");
    assert_eq!(mit.learned, 1);
    assert_eq!(etiketten(&registry), vec![paar(STROM, "Sonstiges")]);
}

#[test]
fn ein_widerspruch_weist_die_ganze_datei_ab_mit_positionen_aus_der_datei() {
    let db = TempDbPath::new("import_widerspruch");
    let registry = register(&db);
    // Der Widerspruch steht bei einem **nie ausgegebenen** Schlüssel — geprüft wird er trotzdem,
    // vor jedem Filter, über alle formal gültigen Einträge.
    let antwort = AntwortDatei::mit(
        "widerspruch",
        &format!(
            r#"{{"antworten": [
                {{"fingerabdruck": "{STROM}", "kategorie": "Energie"}},
                {{"fingerabdruck": "COMPANY:ERFUNDEN", "kategorie": "A"}},
                {{"kategorie": "ohne Schlüssel"}},
                {{"fingerabdruck": "COMPANY:ERFUNDEN", "kategorie": "B"}}
            ]}}"#
        ),
    );

    let fehler = import(&registry, &antwort.0, ImportMode::LearnUnknown);

    assert_eq!(
        fehler,
        Err(ImportError::Contradiction {
            first: 2,
            second: 4
        })
    );
    assert_eq!(etiketten(&registry), vec![], "nichts gelernt");
}

#[test]
fn eine_ungueltige_kategorie_verwirft_den_eintrag() {
    let db = TempDbPath::new("import_kategorie");
    let registry = register(&db);
    let zu_lang = "x".repeat(61);
    let antwort = AntwortDatei::mit(
        "kategorie",
        &format!(
            r#"{{"antworten": [
                {{"fingerabdruck": "{STROM}", "kategorie": "Zeile\nzwei"}},
                {{"fingerabdruck": "{VERSICHERUNG}", "kategorie": "{zu_lang}"}},
                {{"fingerabdruck": "{MUSIK}", "kategorie": "   "}},
                {{"fingerabdruck": "{MUSIK}", "kategorie": 7}}
            ]}}"#
        ),
    );

    let bericht = import(&registry, &antwort.0, ImportMode::LearnUnknown).expect("gelesen");

    assert_eq!(
        bericht.discarded,
        vec![
            (1, DiscardCause::InvalidCategory),
            (2, DiscardCause::InvalidCategory),
            (3, DiscardCause::InvalidCategory),
            (4, DiscardCause::MissingCategory),
        ]
    );
    assert_eq!(bericht.learned, 0);
}

#[test]
fn die_kategorie_ist_einzeilig_ohne_steuerzeichen_und_hoechstens_sechzig_zeichen_lang() {
    assert!(Kategorie::parse("Spiel & Spaß").is_some());
    assert!(Kategorie::parse(&"ä".repeat(60)).is_some());
    for ungueltig in [
        "",
        "  ",
        "a\tb",
        "a\u{2028}b",
        "a\u{202E}b",
        "a\u{7}b",
        &"ä".repeat(61),
    ] {
        assert_eq!(Kategorie::parse(ungueltig), None);
    }
}

#[test]
fn kaputtes_json_nennt_nur_zeile_und_spalte_nie_den_inhalt() {
    let db = TempDbPath::new("import_kaputt");
    let registry = register(&db);
    let antwort = AntwortDatei::mit("kaputt", "{\"antworten\": [\n  {\"geheim-wert\" 42}\n]}");

    let fehler =
        import(&registry, &antwort.0, ImportMode::LearnUnknown).expect_err("kein gültiges JSON");

    assert!(matches!(fehler, ImportError::Malformed { line: 2, .. }));
    assert!(!fehler.to_string().contains("geheim"));
}

#[test]
fn ohne_antworten_liste_wird_die_datei_abgewiesen() {
    let db = TempDbPath::new("import_ohne_liste");
    let registry = register(&db);
    let antwort = AntwortDatei::mit(
        "ohne_liste",
        r#"[{"fingerabdruck": "x", "kategorie": "y"}]"#,
    );

    assert_eq!(
        import(&registry, &antwort.0, ImportMode::LearnUnknown),
        Err(ImportError::NoAnswerList)
    );
}

#[test]
fn eine_zu_grosse_datei_wird_nicht_geparst() {
    let db = TempDbPath::new("import_gross");
    let registry = register(&db);
    let gross = " ".repeat(usize::try_from(MAX_ANSWER_BYTES).unwrap_or(usize::MAX)) + "{}";
    let antwort = AntwortDatei::mit("gross", &gross);

    assert_eq!(
        import(&registry, &antwort.0, ImportMode::LearnUnknown),
        Err(ImportError::TooLarge)
    );
}
