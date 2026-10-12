//! Der `Speicher` über echte Läufe ([Issue 0009](../../../../docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)):
//! was über mehrere Importe und Neustarts in **einer** SQLite-Datei liegt, was davon wieder geladen
//! wird — und was dort **nie** liegt (kein Klarname außerhalb der Rohtabelle, nirgends der Seed).
//!
//! Ein **Neustart** ist hier, was er im Programm ist: jeder [`run`] öffnet die Datei neu über den
//! Schema-Besitzer und lässt sie am Ende fallen. Alle Daten sind erfunden.

use std::cell::RefCell;

use auswerten_adapter::{
    CategorizerSeam, CategorizerUnavailable, CategoryAnswerRow, CategoryRequestRow, OccurrenceSeam,
};
use rusqlite::Connection;

use super::*;

use crate::auswerten::{SqliteCategoryRegistry, consumer};
use crate::outbox::OutboxMessage;
use crate::pipeline_test_support::TempDbPath;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR")))
}

/// Die Pfade eines Laufs über den Auszug `csv` mit den Lauf-Configs.
fn pfade(db: &TempDbPath, csv: &str) -> RunPaths {
    RunPaths {
        database: db.path().to_path_buf(),
        transactions: fixture(csv),
        window_config: fixture("lauf/fenster.toml"),
        seed_config: fixture("lauf/seed.toml"),
        own_accounts_config: fixture("lauf/konten.toml"),
        persons_config: fixture("lauf/namen.toml"),
    }
}

fn lauf(paths: &RunPaths) {
    run(paths, &mut Vec::new()).expect("der Lauf gelingt");
}

/// Öffnet die Datei so, wie ein Programmstart sie öffnet.
fn neustart(paths: &RunPaths) -> SqliteOutbox {
    schema::open(&AppConfig::new(&paths.database)).expect("die Lauf-Datei lässt sich öffnen")
}

/// Der jüngste Lauf und die Spanne seiner Fenster.
fn juengster_lauf(conn: &Connection) -> rusqlite::Result<(String, i64, i64)> {
    conn.query_row(
        "SELECT run_id, min(from_day), max(to_day) FROM auswertungsfenster \
         WHERE run_id = (SELECT max(run_id) FROM auswertungsfenster);",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
}

/// Jede Zelle jeder Tabelle als Text, je Tabelle gebündelt — gelesen am Programm vorbei, über die
/// Liste, die SQLite selbst führt (nicht über die Deklaration des Schema-Besitzers).
fn alle_zellen(conn: &Connection) -> rusqlite::Result<Vec<(String, Vec<String>)>> {
    let tabellen = {
        let mut statement =
            conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name;")?;
        let namen = statement.query_map([], |row| row.get::<_, String>(0))?;
        namen.collect::<rusqlite::Result<Vec<_>>>()?
    };
    tabellen
        .into_iter()
        .map(|tabelle| {
            let zellen = spalten(conn, &tabelle)?
                .iter()
                .map(|spalte| zellen_der_spalte(conn, &tabelle, spalte))
                .collect::<rusqlite::Result<Vec<_>>>()?
                .concat();
            Ok((tabelle, zellen))
        })
        .collect()
}

fn zellen_der_spalte(
    conn: &Connection,
    tabelle: &str,
    spalte: &str,
) -> rusqlite::Result<Vec<String>> {
    let mut statement = conn.prepare(&format!(
        "SELECT coalesce(CAST({spalte} AS TEXT), '') FROM {tabelle};"
    ))?;
    let werte = statement.query_map([], |row| row.get::<_, String>(0))?;
    werte.collect()
}

fn spalten(conn: &Connection, tabelle: &str) -> rusqlite::Result<Vec<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({tabelle});"))?;
    let namen = statement.query_map([], |row| row.get(1))?;
    namen.collect()
}

// --- AK 1: zwei Importe, ein Neustart, alles wieder da ----------------------------------------

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn zwei_importe_werden_persistiert_und_beim_neustart_vollstaendig_geladen()
-> Result<(), Box<dyn std::error::Error>> {
    // Zwei getrennte CSV-Importe (zwei eigene Konten) in eine Datei — jeder in seinem eigenen Lauf,
    // also mit eigenem Öffnen und Schließen.
    let db = TempDbPath::new("speicher_zwei_importe");
    lauf(&pfade(&db, "einlesen/csv/lauf.csv"));
    let paths = pfade(&db, "einlesen/csv/lauf_haushalt.csv");
    lauf(&paths);

    // Neustart: die Datei frisch öffnen und den Bestand über den **realen** Lese-Weg des Programms
    // laden — die obfuskierten Buchungen, die der jüngste Lauf in seine Fenster gelegt hat.
    let outbox = neustart(&paths);
    let (lauf_id, von, bis) = outbox.with_connection(juengster_lauf)?;
    let geladen = AuswertungsfensterSource::new(outbox.clone()).history_in(&lauf_id, von, bis)?;

    let gespeichert: i64 = outbox.with_connection(|conn| {
        conn.query_row("SELECT count(*) FROM obfuskiert;", [], |row| row.get(0))
    })?;
    // 53 gebuchte Zeilen des ersten, 8 des zweiten Auszugs (dessen vorgemerkte Zeile zählt nicht).
    assert_eq!(gespeichert, 53 + 8);
    assert_eq!(i64::try_from(geladen.bookings.len())?, gespeichert);
    // Und es sind **beide** Importe: die Buchungen tragen die Token beider Auftragskonten.
    let konten = geladen
        .bookings
        .iter()
        .map(|booking| booking.ordering_account.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(konten.len(), 2, "{konten:?}");
    assert!(
        konten.iter().all(|konto| konto.starts_with("KONTO_")),
        "{konten:?}"
    );
    Ok(())
}

// --- AK 2: eine gelernte Korrektur übersteht den Neustart und wird angewendet ------------------

/// Ein Orakel, das **mitschreibt**, wonach es gefragt wurde, und nichts antwortet.
#[derive(Default)]
struct MitschreibendesOrakel {
    gefragt: RefCell<Vec<String>>,
}

impl CategorizerSeam for MitschreibendesOrakel {
    fn ask(
        &self,
        requests: &[CategoryRequestRow],
    ) -> Result<Vec<CategoryAnswerRow>, CategorizerUnavailable> {
        self.gefragt
            .borrow_mut()
            .extend(requests.iter().map(|request| request.fingerprint.clone()));
        Ok(Vec::new())
    }
}

/// Wertet jedes Fenster des jüngsten Laufs **erneut** aus — über den realen `auswerten`-Consumer, das
/// reale Register und die reale Lese-Quelle, nur mit dem mitschreibenden Orakel — und gibt zurück,
/// wonach das Orakel gefragt wurde.
fn gefragte_arten(paths: &RunPaths) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let outbox = neustart(paths);
    let seed = SeedConfig::load(&paths.seed_config)?.seed();
    let own_accounts =
        EigeneKontenConfig::load(&paths.own_accounts_config)?.to_own_accounts_config(seed);
    let fenster: Vec<String> = outbox.with_connection(|conn| {
        let mut statement = conn.prepare(
            "SELECT id FROM auswertungsfenster \
             WHERE run_id = (SELECT max(run_id) FROM auswertungsfenster) ORDER BY id;",
        )?;
        let ids = statement.query_map([], |row| row.get(0))?;
        ids.collect()
    })?;
    let quelle = AuswertungsfensterSource::new(outbox.clone());
    let kategorien = SqliteCategoryRegistry::new(outbox.clone());
    let orakel = MitschreibendesOrakel::default();
    let mut dispatcher = Dispatcher::new();
    register(
        &mut dispatcher,
        AUSWERTEN_CONSUMED,
        Box::new(consumer(
            &quelle,
            &kategorien,
            &orakel,
            own_accounts,
            AuswertenSqliteStore::new(outbox.clone()),
        )),
    )?;
    for id in &fenster {
        outbox.enqueue(AUSWERTEN_CONSUMED, id)?;
    }
    let ergebnisse: Vec<(OutboxMessage, _)> = outbox.deliver_open(&dispatcher)?;
    if ergebnisse.len() != fenster.len() || ergebnisse.iter().any(|(_, ok)| ok.is_err()) {
        return Err("die erneute Auswertung der Fenster ist gescheitert".into());
    }
    let mut gefragt = orakel.gefragt.take();
    gefragt.sort();
    gefragt.dedup();
    Ok(gefragt)
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
)]
#[test]
fn eine_gelernte_korrektur_uebersteht_den_neustart_und_das_orakel_fragt_nicht_mehr_danach()
-> Result<(), Box<dyn std::error::Error>> {
    let db = TempDbPath::new("speicher_korrektur");
    let paths = pfade(&db, "einlesen/csv/lauf.csv");
    lauf(&paths);

    // Vorher: ein leeres Register — jede Art mit Kategorie-Signal geht ans Orakel.
    let vorher = gefragte_arten(&paths)?;
    let Some(gelernt) = vorher.first().cloned() else {
        return Err("der Lauf-Bestand muss mindestens eine Art fürs Orakel liefern".into());
    };

    // Eine Korrektur lernen (der Weg, den der Import der Chat-Antwort nimmt) — und die Datei
    // schließen.
    SqliteCategoryRegistry::new(neustart(&paths)).learn(&[CategoryAnswerRow {
        fingerprint: gelernt.clone(),
        category: "Haushalt".to_string(),
    }])?;

    // Nachher, nach einem Neustart: die gelernte Art erreicht das Orakel nicht mehr, alle anderen
    // weiterhin.
    let nachher = gefragte_arten(&paths)?;
    let erwartet = vorher
        .iter()
        .filter(|art| **art != gelernt)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(nachher, erwartet);
    assert_eq!(
        SqliteCategoryRegistry::new(neustart(&paths))
            .categories_of(std::slice::from_ref(&gelernt))?,
        vec![CategoryAnswerRow {
            fingerprint: gelernt,
            category: "Haushalt".to_string(),
        }]
    );
    Ok(())
}

// --- AK 4: kein Klarname außerhalb der Rohtabelle ----------------------------------------------

/// Die erfundenen Personen der Scrubbing-Fixture: zwei Empfänger ohne Gläubiger-ID und ein Name, der
/// nur im Verwendungszweck steht.
const KLARNAMEN: [&str; 3] = ["lisa beispiel", "paul erfunden", "oma fiktiv"];

/// Die eine Tabelle, die Rohdaten lokal halten **darf** (ADR-0008/ADR-0010): `einlesen` legt die
/// Zeile so ab, wie sie im Auszug steht. Alles stromabwärts von `obfuskieren` — und das Register —
/// trägt nur noch Token.
const ROHTABELLE: &str = "einlesen_lines";

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
)]
#[test]
fn kein_klarname_steht_ausserhalb_der_rohtabelle_auch_nicht_im_register()
-> Result<(), Box<dyn std::error::Error>> {
    let db = TempDbPath::new("speicher_klarnamen");
    let paths = pfade(&db, "einlesen/csv/scrubbing.csv");
    lauf(&paths);
    // Das Register trägt, was ein Import lernen würde: ein Etikett zu einer Personen-Art.
    let outbox = neustart(&paths);
    let person: String = outbox.with_connection(|conn| {
        conn.query_row(
            "SELECT fingerprint FROM window_evaluation_fingerprints \
             WHERE fingerprint LIKE 'PERSON:%' LIMIT 1;",
            [],
            |row| row.get(0),
        )
    })?;
    SqliteCategoryRegistry::new(outbox.clone()).learn(&[CategoryAnswerRow {
        fingerprint: person,
        category: "Familie".to_string(),
    }])?;

    let tabellen = outbox.with_connection(alle_zellen)?;

    for (tabelle, zellen) in &tabellen {
        let text = zellen.join("\n").to_lowercase();
        match tabelle.as_str() {
            // Gegenprobe: die Rohtabelle trägt die Namen tatsächlich — die Suche greift also.
            ROHTABELLE => assert!(text.contains(KLARNAMEN[0]), "{tabelle}"),
            _ => {
                for name in KLARNAMEN {
                    assert!(!text.contains(name), "`{name}` steht roh in {tabelle}");
                }
            }
        }
    }
    assert!(
        tabellen
            .iter()
            .any(|(tabelle, zellen)| tabelle == "learned_categories" && zellen.len() > 1),
        "das Register ist Teil der Prüfung"
    );
    Ok(())
}

// --- AK 5: der Speicher trägt den Seed nicht ----------------------------------------------------

/// Der markante Seed der Fixture `lauf/seed_markant.toml` — eine Zahl, die sonst nirgends vorkommt.
const MARKANTER_SEED: i64 = 8_675_309_421;

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
)]
#[test]
fn weder_register_noch_eine_andere_tabelle_traegt_den_seed()
-> Result<(), Box<dyn std::error::Error>> {
    let db = TempDbPath::new("speicher_seed");
    let paths = RunPaths {
        seed_config: fixture("lauf/seed_markant.toml"),
        ..pfade(&db, "einlesen/csv/lauf.csv")
    };
    lauf(&paths);
    let outbox = neustart(&paths);
    let art: String = outbox.with_connection(|conn| {
        conn.query_row(
            "SELECT fingerprint FROM window_evaluation_fingerprints LIMIT 1;",
            [],
            |row| row.get(0),
        )
    })?;
    SqliteCategoryRegistry::new(outbox.clone()).learn(&[CategoryAnswerRow {
        fingerprint: art,
        category: "Energie".to_string(),
    }])?;

    let seed = MARKANTER_SEED.to_string();
    for (tabelle, zellen) in outbox.with_connection(alle_zellen)? {
        // Keine Spalte heißt so — der Marker `seed_marker` ist ein Hash, kein Seed.
        assert!(
            !outbox
                .with_connection(|conn| spalten(conn, &tabelle))?
                .iter()
                .any(|spalte| spalte == "seed"),
            "{tabelle} hat eine Seed-Spalte"
        );
        assert!(
            !zellen.iter().any(|zelle| zelle.contains(&seed)),
            "der Seed steht in {tabelle}"
        );
    }
    drop(outbox);
    // Und die Datei als Ganzes, Byte für Byte, trägt ihn auch nicht als Text.
    let datei = std::fs::read(&paths.database)?;
    assert!(
        !datei
            .windows(seed.len())
            .any(|stelle| stelle == seed.as_bytes()),
        "der Seed steht in der Datei"
    );
    Ok(())
}
