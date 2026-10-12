//! Der Chat-Kategorisierer über **echte** Läufe
//! ([Issue 0011](../../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md)): was der Export
//! enthält — und vor allem, was **nie** —, der Round-Trip über den Import und dass eine Korrektur
//! bleibt. Echte SQLite-Datei, echte CSV-Quelle, echte Loader; alle Daten sind erfunden.

use serde_json::Value;

use super::*;

use crate::auswerten::chat_import::{DiscardCause, ImportMode};
use crate::pipeline_test_support::TempDbPath;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR")))
}

/// Eine Datei im Temp-Verzeichnis, die nach dem Test verschwindet.
struct TempDatei(PathBuf);

impl TempDatei {
    fn neu(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|dauer| dauer.as_nanos())
            .unwrap_or(0);
        Self(std::env::temp_dir().join(format!(
            "bank_auswertung_kategorien_{tag}_{}_{nanos}.json",
            std::process::id()
        )))
    }

    fn mit(tag: &str, inhalt: &str) -> Self {
        let datei = Self::neu(tag);
        std::fs::write(&datei.0, inhalt).expect("die Datei lässt sich schreiben");
        datei
    }
}

impl Drop for TempDatei {
    fn drop(&mut self) {
        let _aufraeumen: std::io::Result<()> = std::fs::remove_file(&self.0);
    }
}

/// Der markante Seed der Fixture `lauf/seed_markant.toml`.
const SEED: u64 = 8_675_309_421;

/// Die Pfade eines Laufs über `csv` mit Export nach `export`.
fn pfade(db: &TempDbPath, csv: &str, export: &TempDatei) -> RunPaths {
    RunPaths {
        database: db.path().to_path_buf(),
        transactions: fixture(csv),
        window_config: fixture("lauf/fenster.toml"),
        seed_config: fixture("lauf/seed_markant.toml"),
        own_accounts_config: fixture("lauf/konten.toml"),
        persons_config: fixture("lauf/namen_kategorien.toml"),
        savings_config: fixture("lauf/sparen.toml"),
        category_export: Some(export.0.clone()),
        report: db.report_path(),
    }
}

fn lauf(paths: &RunPaths) {
    run(paths, &mut Vec::new()).expect("der Lauf gelingt");
}

/// Die Anfragen der Export-Datei als Paare (Fingerabdruck, Text) — und zugleich der Beleg, dass jeder
/// Eintrag **genau** diese zwei Felder trägt.
fn anfragen(export: &TempDatei) -> Vec<(String, String)> {
    let inhalt: Value =
        serde_json::from_str(&std::fs::read_to_string(&export.0).expect("die Export-Datei ist da"))
            .expect("gültiges JSON");
    inhalt
        .get("anfragen")
        .and_then(Value::as_array)
        .expect("eine Anfrage-Liste")
        .iter()
        .map(|eintrag| {
            let objekt = eintrag.as_object().expect("ein Objekt je Anfrage");
            assert_eq!(objekt.len(), 2, "genau zwei Felder je Anfrage");
            (
                text_feld(objekt, "fingerabdruck"),
                text_feld(objekt, "text"),
            )
        })
        .collect()
}

fn text_feld(objekt: &serde_json::Map<String, Value>, feld: &str) -> String {
    objekt
        .get(feld)
        .and_then(Value::as_str)
        .expect("ein Textfeld")
        .to_string()
}

fn paar(fingerprint: &str, text: &str) -> (String, String) {
    (fingerprint.to_string(), text.to_string())
}

/// Großbuchstaben, ohne jedes Trennzeichen — so fällt ein Wert auch auf, wenn er zerlegt und neu
/// verbunden wurde (`max.mustermann` → `MAX-MUSTERMANN` → `MAXMUSTERMANN`).
fn normalisiert(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

const STROM: &str = "COMPANY:DE98ZZZ09999999999";
const VERSICHERUNG: &str = "COMPANY:DE13ZZZ00000000123";
const PRAXIS: &str = "COMPANY:DE95ZZZ00000000005";
const MUSIK: &str = "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:PERSON-MUSIKDIENST-BEISPIEL";
const KINO: &str = "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:FIKTNLAA-KINOKASSE-NORD";

/// Alle Rohwerte der Fixture `kategorien.csv` (und ihrer Namensliste), die die Maschine nie verlassen
/// dürfen: IBANs (Auftrags- und Gegenkonten, auch die im Zweck), BICs, Klarnamen, E-Mail,
/// Kunden-/Mandats-/End-to-End-Referenzen.
const ROHWERTE: [&str; 29] = [
    "DE93100000000000000100",
    "DE75512108001245126199",
    "DE75 5121 0800 1245 1261 99",
    "LU280019400644750000",
    "DE02100100100006820101",
    "DE89370400440532013000",
    "DE07100100100001234567",
    "DE51100100100007654321",
    "DE68100100100000000777",
    "DE85100100100000000123",
    "DE19100100100000000555",
    "BYLADEM1001",
    "FIKTLULL",
    "FIKTDEFFXXX",
    "FIKTDEFF002",
    "Erika Beispiel",
    "Lisa Fiktiv",
    "max.mustermann@web.example",
    "Max Mustermann",
    "HANS VERMIETER",
    "KLAUS FREMDKONTO",
    "PETRA SPALTE",
    "VEREIN FIKTIV",
    "Kd-Nr 4711",
    "4711",
    "MANDAT-STROM-7",
    "E2E-KAT",
    "Wohnung 3 OG",
    "Rueckzahlung",
];

#[test]
fn der_export_traegt_nur_unbekannte_firmen_und_durchleiter_und_keinen_rohwert() {
    let db = TempDbPath::new("kategorien_leak");
    let export = TempDatei::neu("leak");
    let paths = pfade(&db, "einlesen/csv/kategorien.csv", &export);
    let mut protokoll = Vec::new();

    run(&paths, &mut protokoll).expect("der Lauf gelingt");

    // Firmen nur mit ihrem Namen (gescrubbt: „Lisa Fiktiv" steht auf der Namensliste), Durchleiter nur
    // mit ihrem Zweck — ohne Wörter mit Ziffer oder `@`. **Nicht** dabei: PayPal ohne Zweck, die Firma
    // ohne Namen, die Person, und beide Zeilen mit unbrauchbarer Gläubiger-Spalte (IBAN bzw. Name —
    // sie sind Personen).
    assert_eq!(
        anfragen(&export),
        vec![
            paar(VERSICHERUNG, "VERSICHERUNG FIKTIV AG"),
            paar(PRAXIS, "PRAXIS [PERSON]"),
            paar(STROM, "STADTWERKE BEISPIELSTADT"),
            paar(KINO, "FIKTNLAA Kinokasse Nord"),
            paar(MUSIK, "[PERSON] Musikdienst Beispiel"),
        ]
    );

    let roh = std::fs::read_to_string(&export.0).expect("lesbar");
    let flach = normalisiert(&roh);
    for wert in ROHWERTE {
        assert!(!roh.contains(wert), "Rohwert im Export: {wert}");
        assert!(
            !flach.contains(&normalisiert(wert)),
            "Rohwert (normalisiert) im Export: {wert}"
        );
    }
    // Weder der Seed (dezimal, hex) noch ein von ihm abgeleiteter Wert: kein Konto-/Personen-Token
    // und nicht der Seed-Marker aus `obfuskiert` — jeweils auch als bloße 32 Hex-Zeichen.
    assert!(!flach.contains(&SEED.to_string()));
    assert!(!flach.contains(&normalisiert(&format!("{SEED:x}"))));
    assert!(!flach.contains("KONTO"));
    for abgeleitet in seed_abgeleitete_werte(&paths) {
        assert!(
            !flach.contains(&normalisiert(&abgeleitet)),
            "seed-abgeleiteter Wert im Export"
        );
        let hex = abgeleitet
            .rsplit('_')
            .next()
            .unwrap_or(&abgeleitet)
            .to_string();
        assert!(!flach.contains(&normalisiert(&hex)), "Token-Hex im Export");
    }

    // Ehrliche Gegenprobe (Lücke aus 0010): eine ausländische BIC ohne Ziffer und ohne `BIC`-Hinweis
    // erkennt der Scrubber nicht — sie steht im Export.
    assert!(roh.contains("FIKTNLAA"));

    // Das Protokoll nennt die Zahl, nie einen Inhalt.
    let protokoll = String::from_utf8(protokoll).expect("UTF-8");
    assert!(protokoll.contains("Kategorien-Export: 5 unbekannte Buchungsart(en)"));
    assert!(!protokoll.contains("COMPANY:") && !protokoll.contains("STADTWERKE"));
}

/// Alle Konto-/Personen-Token und Seed-Marker, die `obfuskieren` abgelegt hat.
fn seed_abgeleitete_werte(paths: &RunPaths) -> Vec<String> {
    let conn = rusqlite::Connection::open(&paths.database).expect("die Datei ist da");
    let mut statement = conn
        .prepare(
            "SELECT ordering_account, counter_account, recipient_pseudonym, seed_marker \
             FROM obfuskiert;",
        )
        .expect("die Abfrage");
    let zeilen = statement
        .query_map([], |row| {
            Ok([
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ])
        })
        .expect("die Zeilen")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("lesbar");
    let werte = zeilen.into_iter().flatten().flatten().collect::<Vec<_>>();
    assert!(werte.len() > 20, "die Fixture trägt Token und Marker");
    werte
}

#[test]
fn der_round_trip_lernt_ueber_den_fingerabdruck_und_der_naechste_export_fragt_nur_den_rest() {
    let db = TempDbPath::new("kategorien_round_trip");
    let export = TempDatei::neu("round_trip");
    let paths = pfade(&db, "einlesen/csv/kategorien.csv", &export);
    lauf(&paths);
    let import_pfade = ImportPaths {
        database: paths.database.clone(),
        answers: fixture("kategorien/antwort.json"),
    };
    let mut protokoll = Vec::new();

    let bericht = import_categories(&import_pfade, ImportMode::LearnUnknown, &mut protokoll)
        .expect("die kanonische Antwort wird gelesen");

    // Der Text als Schlüssel, andere Groß-/Kleinschreibung, Text ohne Fingerabdruck → verworfen;
    // der richtige Fingerabdruck mit Zusatzfeld → gelernt.
    assert_eq!(bericht.learned, 3);
    assert_eq!(
        bericht.discarded,
        vec![
            (2, DiscardCause::NotIssued),
            (3, DiscardCause::NotIssued),
            (4, DiscardCause::MissingFingerprint),
        ]
    );
    assert_eq!(bericht.ignored_fields, 1);
    let protokoll = String::from_utf8(protokoll).expect("UTF-8");
    for inhalt in [
        "COMPANY",
        "STADTWERKE",
        "Wohnen",
        "Versicherung",
        "Unterhaltung",
    ] {
        assert!(!protokoll.contains(inhalt), "Inhalt im Import-Protokoll");
    }

    let registry = SqliteCategoryRegistry::new(neustart(&paths));
    assert_eq!(
        etiketten(&registry),
        vec![
            paar(STROM, "Wohnen & Energie"),
            paar(VERSICHERUNG, "Versicherung"),
            paar(MUSIK, "Unterhaltung"),
        ]
    );

    // Der nächste Lauf kennt die drei — sie stehen nicht mehr im Export.
    lauf(&paths);
    assert_eq!(
        anfragen(&export),
        vec![
            paar(PRAXIS, "PRAXIS [PERSON]"),
            paar(KINO, "FIKTNLAA Kinokasse Nord"),
        ]
    );
}

#[test]
fn ein_bekanntes_etikett_bleibt_ohne_korrektur_und_eine_korrektur_ueberdauert_den_lauf() {
    let db = TempDbPath::new("kategorien_korrektur");
    let export = TempDatei::neu("korrektur");
    let paths = pfade(&db, "einlesen/csv/kategorien.csv", &export);
    lauf(&paths);
    let erste = TempDatei::mit(
        "erste",
        &format!(r#"{{"antworten": [{{"fingerabdruck": "{STROM}", "kategorie": "Energie"}}]}}"#),
    );
    let zweite = TempDatei::mit(
        "zweite",
        &format!(r#"{{"antworten": [{{"fingerabdruck": "{STROM}", "kategorie": "Wohnen"}}]}}"#),
    );
    let mit = |antwort: &TempDatei| ImportPaths {
        database: paths.database.clone(),
        answers: antwort.0.clone(),
    };

    import_categories(&mit(&erste), ImportMode::LearnUnknown, &mut Vec::new()).expect("gelernt");
    let ohne = import_categories(&mit(&zweite), ImportMode::LearnUnknown, &mut Vec::new())
        .expect("gelesen");
    assert_eq!((ohne.learned, ohne.kept_known), (0, vec![1]));
    assert_eq!(
        etiketten(&SqliteCategoryRegistry::new(neustart(&paths))),
        vec![paar(STROM, "Energie")]
    );

    import_categories(&mit(&zweite), ImportMode::Correct, &mut Vec::new()).expect("korrigiert");
    lauf(&paths);
    assert_eq!(
        etiketten(&SqliteCategoryRegistry::new(neustart(&paths))),
        vec![paar(STROM, "Wohnen")],
        "der Lauf schreibt das Register nie"
    );
}

#[test]
fn nichts_gelernt_aber_verworfen_ist_ein_fehlschlag() {
    let db = TempDbPath::new("kategorien_nichts");
    let export = TempDatei::neu("nichts");
    let paths = pfade(&db, "einlesen/csv/kategorien.csv", &export);
    lauf(&paths);
    let antwort = TempDatei::mit(
        "nichts",
        r#"{"antworten": [{"fingerabdruck": "Stadtwerke", "kategorie": "Energie"}]}"#,
    );

    let ergebnis = import_categories(
        &ImportPaths {
            database: paths.database.clone(),
            answers: antwort.0.clone(),
        },
        ImportMode::LearnUnknown,
        &mut Vec::new(),
    );

    assert_eq!(
        ergebnis.map(drop),
        Err(RunError::NothingLearned { discarded: 1 })
    );
}

#[test]
fn ein_liegengebliebener_alter_lauf_steht_nicht_im_export() {
    let db = TempDbPath::new("kategorien_alter_lauf");
    let export = TempDatei::neu("alter_lauf");
    // Lauf 1 über den alten Auszug: die Firma heißt dort „ALTER NAME GMBH".
    let alt = RunPaths {
        category_export: None,
        ..pfade(&db, "einlesen/csv/kategorien_alt.csv", &export)
    };
    lauf(&alt);
    // Ein Absturz ließ die Zustellung eines seiner Fenster offen.
    let outbox = neustart(&alt);
    let fenster: String = outbox
        .with_connection(|conn| {
            conn.query_row("SELECT id FROM auswertungsfenster LIMIT 1;", [], |row| {
                row.get(0)
            })
        })
        .expect("ein Fenster von Lauf 1");
    outbox
        .enqueue(AUSWERTEN_CONSUMED, &fenster)
        .expect("eine offene Nachricht des alten Laufs");
    drop(outbox);

    // Lauf 2 über den neuen Auszug, in dem dieselbe Firma „NEUER NAME GMBH" heißt. Phase 1 führt den
    // alten Lauf zu Ende (dessen Fenster kennt nur den alten Namen), Phase 2 wertet den neuen aus —
    // jüngstes Fenster zuerst.
    lauf(&pfade(&db, "einlesen/csv/kategorien_neu.csv", &export));

    // Hätte der Kategorisierer schon in Phase 1 gesammelt, gewönne der alte Text.
    assert_eq!(
        anfragen(&export),
        vec![paar("COMPANY:DE52ZZZ00000000003", "NEUER NAME GMBH")]
    );
}

#[test]
fn ein_gescheiterter_lauf_schreibt_keinen_export_und_laesst_die_alte_datei() {
    let db = TempDbPath::new("kategorien_gescheitert");
    let export = TempDatei::mit("gescheitert", "alt");

    let ergebnis = run(
        &pfade(&db, "einlesen/csv/pflichtfeld_fehlt.csv", &export),
        &mut Vec::new(),
    );

    assert!(ergebnis.is_err(), "der Lauf scheitert");
    assert_eq!(std::fs::read_to_string(&export.0).expect("lesbar"), "alt");
}

fn neustart(paths: &RunPaths) -> SqliteOutbox {
    schema::open(&AppConfig::new(&paths.database)).expect("die Lauf-Datei lässt sich öffnen")
}

fn etiketten(registry: &SqliteCategoryRegistry) -> Vec<(String, String)> {
    registry
        .categories_of(&[STROM, VERSICHERUNG, MUSIK, PRAXIS, KINO].map(ToString::to_string))
        .expect("das Register ist lesbar")
        .into_iter()
        .map(|answer| (answer.fingerprint, answer.category))
        .collect()
}

#[test]
fn eine_alt_zeile_ohne_gueltige_glaeubiger_id_beendet_den_lauf_ohne_export() {
    let db = TempDbPath::new("kategorien_alt_zeile");
    let export = TempDatei::neu("alt_zeile");
    let paths = pfade(&db, "einlesen/csv/kategorien.csv", &export);
    lauf(&RunPaths {
        category_export: None,
        ..paths.clone()
    });
    // Eine Zeile, wie sie ein älterer Programmstand ablegen konnte: in der Gläubiger-Spalte einer
    // Firma steht eine IBAN. `obfuskieren` lässt so etwas heute nicht mehr entstehen.
    rusqlite::Connection::open(&paths.database)
        .and_then(|conn| {
            conn.execute(
                "UPDATE obfuskiert SET recipient_creditor_id = 'DE89370400440532013000' \
                 WHERE recipient_creditor_id = 'DE98ZZZ09999999999';",
                [],
            )
        })
        .expect("die Alt-Zeile");

    let fehler = run(&paths, &mut Vec::new()).expect_err("der Lauf scheitert terminal");

    let meldung = fehler.to_string();
    assert!(meldung.contains("frische --db"), "Hinweis fehlt: {meldung}");
    assert!(!meldung.contains("DE89370400440532013000"));
    assert!(
        !export.0.exists(),
        "kein Export nach einem gescheiterten Lauf"
    );
}

#[test]
fn beide_slices_urteilen_gleich_ueber_eine_glaeubiger_id() {
    // Die Domänen sind blind füreinander; die Regel steht darum zweimal. Diese Tabelle hält fest, dass
    // beide Kopien gleich urteilen.
    for (kandidat, gueltig) in [
        ("DE98ZZZ09999999999", true),
        ("LU96ZZZ0000000000000000058", true),
        ("DE94ZZZ00000561653", true),
        ("DE13ABC00000000123", true),
        ("DE00ZZZ00000000123", false),
        ("DE89370400440532013000", false),
        ("Hans Vermieter", false),
        ("de98zzz09999999999", false),
        ("DE98 ZZZ 09999999999", false),
        ("DE98ZZZ", false),
        ("", false),
    ] {
        assert_eq!(
            obfuskierung_adapter::is_valid_creditor_id(kandidat),
            gueltig,
            "obfuskieren: {kandidat}"
        );
        assert_eq!(
            auswerten_adapter::is_valid_creditor_id(kandidat),
            gueltig,
            "auswerten: {kandidat}"
        );
    }
}
