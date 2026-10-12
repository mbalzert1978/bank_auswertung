//! Das Kategorie-Register gegen **echtes** SQLite (Issue 0009): Lernen, Korrigieren, Neustart,
//! Redaktion. Ein Neustart ist hier, was er im Programm ist — das Rückgrat fallen lassen und die Datei
//! über den Schema-Besitzer neu öffnen.

use super::*;

use crate::config::AppConfig;
use crate::pipeline_test_support::TempDbPath;
use crate::schema;

/// Erfundene Fingerabdrücke in der Form, die die Domäne bildet (Firma über Gläubiger-ID, Person über
/// ihr Pseudonym).
const STROM: &str = "COMPANY:DE00ZZZ00000000007";
const STREAMING: &str = "COMPANY:DE00ZZZ00000000042";
const PERSON: &str = "PERSON:PERSON_3";

fn antwort(fingerprint: &str, category: &str) -> CategoryAnswerRow {
    CategoryAnswerRow {
        fingerprint: fingerprint.to_string(),
        category: category.to_string(),
    }
}

fn schluessel(fingerprints: &[&str]) -> Vec<String> {
    fingerprints.iter().map(ToString::to_string).collect()
}

/// Öffnet die Datei so, wie ein Programmstart sie öffnet.
fn oeffne(db: &TempDbPath) -> SqliteCategoryRegistry {
    SqliteCategoryRegistry::new(
        schema::open(&AppConfig::new(db.path())).expect("die Test-Datei lässt sich öffnen"),
    )
}

#[test]
fn eine_gelernte_kategorie_uebersteht_einen_neustart() {
    let db = TempDbPath::new("register_neustart");
    oeffne(&db)
        .learn(&[antwort(STROM, "Energie"), antwort(PERSON, "Familie")])
        .expect("das Register lernt");

    let nach_dem_neustart = oeffne(&db);

    assert_eq!(
        nach_dem_neustart.categories_of(&schluessel(&[STROM, STREAMING, PERSON])),
        Ok(vec![antwort(STROM, "Energie"), antwort(PERSON, "Familie")])
    );
    // Und der Lauf sieht es über seinen Seam: bekannt ist, was gelernt wurde — nichts sonst.
    assert_eq!(
        nach_dem_neustart.known_among(&schluessel(&[STREAMING, STROM])),
        Ok(schluessel(&[STROM]))
    );
}

#[test]
fn eine_korrektur_ersetzt_das_alte_etikett_und_uebersteht_den_neustart() {
    let db = TempDbPath::new("register_korrektur");
    oeffne(&db)
        .learn(&[antwort(STREAMING, "Lebensmittel")])
        .expect("das erste Etikett");

    oeffne(&db)
        .learn(&[antwort(STREAMING, "Spiel & Spaß")])
        .expect("die Korrektur");

    assert_eq!(
        oeffne(&db).categories_of(&schluessel(&[STREAMING])),
        Ok(vec![antwort(STREAMING, "Spiel & Spaß")]),
        "je Art genau eine Kategorie — die zuletzt gelernte"
    );
}

#[test]
fn dasselbe_etikett_erneut_zu_lernen_aendert_nichts() {
    let db = TempDbPath::new("register_wiederholt");
    let register = oeffne(&db);
    let stapel = [antwort(STROM, "Energie")];
    register.learn(&stapel).expect("erstmals");
    let vorher = zeitstempel(&register).expect("die Zeitstempel");

    register.learn(&stapel).expect("ein zweites Mal");

    assert_eq!(
        zeitstempel(&register).expect("die Zeitstempel"),
        vorher,
        "ein unverändertes Etikett rückt auch `updated_at` nicht nach"
    );
}

fn zeitstempel(register: &SqliteCategoryRegistry) -> rusqlite::Result<Vec<(i64, i64)>> {
    register.outbox.with_connection(|conn| {
        let mut statement =
            conn.prepare("SELECT created_at, updated_at FROM learned_categories;")?;
        let zeilen = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        zeilen.collect()
    })
}

#[test]
fn fingerabdruck_und_etikett_werden_getrimmt_abgelegt() {
    let db = TempDbPath::new("register_trim");
    let register = oeffne(&db);

    register
        .learn(&[antwort(&format!("  {STROM} "), " Energie\t")])
        .expect("das Register lernt");

    assert_eq!(
        register.categories_of(&schluessel(&[STROM])),
        Ok(vec![antwort(STROM, "Energie")])
    );
}

#[test]
fn ein_ungueltiger_eintrag_laesst_den_ganzen_stapel_scheitern_und_nennt_keinen_wert() {
    let db = TempDbPath::new("register_leer");
    let register = oeffne(&db);

    let fehler = register.learn(&[antwort(STROM, "Energie"), antwort(PERSON, "   ")]);

    assert_eq!(
        fehler,
        Err(LearnError::Rejected {
            position: 2,
            cause: RejectCause::EmptyCategory,
        })
    );
    assert_eq!(
        register.categories_of(&schluessel(&[STROM])),
        Ok(vec![]),
        "alle oder keines: auch der gültige erste Eintrag ist nicht gelernt"
    );
    let meldung = fehler.err().map(|f| f.to_string()).unwrap_or_default();
    assert!(meldung.contains("Eintrag 2"), "{meldung}");
    assert!(
        !meldung.contains(PERSON) && !meldung.contains(STROM),
        "{meldung}"
    );
}

#[test]
fn ein_leerer_fingerabdruck_wird_abgewiesen() {
    let db = TempDbPath::new("register_ohne_schluessel");
    let register = oeffne(&db);

    assert_eq!(
        register.learn(&[antwort("", "Energie")]),
        Err(LearnError::Rejected {
            position: 1,
            cause: RejectCause::EmptyFingerprint,
        })
    );
}

#[test]
fn zwei_verschiedene_etiketten_fuer_dieselbe_art_in_einem_stapel_werden_abgewiesen() {
    let db = TempDbPath::new("register_widerspruch");
    let register = oeffne(&db);

    let fehler = register.learn(&[
        antwort(STREAMING, "Spiel & Spaß"),
        antwort(STROM, "Energie"),
        antwort(STREAMING, "Lebensmittel"),
    ]);

    assert_eq!(
        fehler,
        Err(LearnError::Rejected {
            position: 3,
            cause: RejectCause::ContradictingCategory,
        })
    );
    assert_eq!(
        register.categories_of(&schluessel(&[STREAMING, STROM])),
        Ok(vec![])
    );
}

#[test]
fn dieselbe_antwort_doppelt_im_stapel_ist_kein_widerspruch() {
    let db = TempDbPath::new("register_doppelt");
    let register = oeffne(&db);

    register
        .learn(&[antwort(STROM, "Energie"), antwort(STROM, "Energie")])
        .expect("eine wiederholte, gleiche Antwort ist harmlos");

    assert_eq!(
        register.categories_of(&schluessel(&[STROM])),
        Ok(vec![antwort(STROM, "Energie")])
    );
}

#[test]
fn ein_nicht_lesbares_register_ist_terminal_und_nennt_keinen_schluessel() {
    // Ein Rückgrat **ohne** die Register-Tabelle: die Auskunft scheitert — und meldet die Mechanik,
    // nie den gefragten Fingerabdruck.
    let outbox = SqliteOutbox::open_in_memory().expect("das Test-Rückgrat");
    let register = SqliteCategoryRegistry::new(outbox);

    let fehler = register.known_among(&schluessel(&[PERSON]));

    assert!(
        matches!(&fehler, Err(RegistryUnavailable { message }) if message.contains("learned_categories")),
        "{fehler:?}"
    );
    let meldung = fehler.err().map(|f| f.message).unwrap_or_default();
    assert!(!meldung.contains(PERSON), "{meldung}");
}
