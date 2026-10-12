use super::*;

use crate::pipeline_test_support::TempDbPath;

/// Die vier Ergebnis-Tabellen der Kette — je Slice die, die sein Ergebnis trägt. Dass **alle
/// vier** nach einem Lauf gefüllt sind, ist die Aussage „die Pipeline lief durch" in
/// nachprüfbarer Form.
const ERGEBNIS_TABELLEN: [&str; 4] = [
    "einlesen_lines",
    "obfuskiert",
    "auswertungsfenster",
    "window_evaluations",
];

/// **Rohwerte der Lauf-CSV**, die in **keiner** Ausgabe auftauchen dürfen: eine eigene IBAN, eine
/// fremde IBAN, ein Empfängername und ein Verwendungszweck. Der Seed steht daneben als Zahl.
const ROHWERTE: [&str; 5] = [
    "DE93100000000000000100",
    "DE10200000000000000101",
    "MUSTERMARKT SE",
    "Bezuege 06/2026",
    "4711",
];

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR")))
}

/// Die Pfade des erfundenen Lauf-Bestands: die reale CSV-Fixture und die vier realen Configs. Die
/// **Mechanik** ist durchweg echt (SQLite-Datei, CSV-Quelle, toml-Loader) — **erfunden** sind
/// allein die Daten.
fn lauf_pfade(db: &TempDbPath) -> RunPaths {
    RunPaths {
        database: db.path().to_path_buf(),
        transactions: fixture("einlesen/csv/lauf.csv"),
        window_config: fixture("lauf/fenster.toml"),
        seed_config: fixture("lauf/seed.toml"),
        own_accounts_config: fixture("lauf/konten.toml"),
        persons_config: fixture("lauf/namen.toml"),
    }
}

/// Zählt über eine **eigene** Verbindung auf die Lauf-Datenbank — der Lauf ist da längst beendet,
/// die Zählung sieht also nur committete Zeilen.
fn zaehle(paths: &RunPaths, sql: &str) -> rusqlite::Result<i64> {
    rusqlite::Connection::open(&paths.database)?.query_row(sql, [], |row| row.get(0))
}

/// Die Zahl noch **offener** Nachrichten — nach einem gelungenen Lauf muss sie null sein.
fn offene_nachrichten(paths: &RunPaths) -> rusqlite::Result<i64> {
    zaehle(
        paths,
        "SELECT count(*) FROM outbox WHERE processed_on_utc IS NULL;",
    )
}

/// Die Zahl der Zeilen einer Ergebnis-Tabelle.
fn zeilen(paths: &RunPaths, table: &str) -> rusqlite::Result<i64> {
    zaehle(paths, &format!("SELECT count(*) FROM {table};"))
}

#[expect(
    clippy::panic_in_result_fn,
    clippy::unwrap_in_result,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert                   trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_lauf_fuellt_alle_vier_ergebnis_tabellen_und_laeuft_die_outbox_leer() -> rusqlite::Result<()>
{
    let db = TempDbPath::new("lauf");
    let paths = lauf_pfade(&db);
    let mut protokoll = Vec::new();

    let report = run(&paths, &mut protokoll).expect("der Lauf über die erfundene CSV gelingt");

    // Kein Fake-Seam war beteiligt: echte SQLite-Datei, echte CSV-Quelle, echte Loader.
    for table in ERGEBNIS_TABELLEN {
        assert!(
            zeilen(&paths, table)? > 0,
            "die Ergebnis-Tabelle {table} muss nach dem Lauf gefüllt sein"
        );
    }
    assert_eq!(
        offene_nachrichten(&paths)?,
        0,
        "ein gelungener Lauf lässt die Outbox leer zurück"
    );

    // Die ganze Kette hat gearbeitet — jeder der sieben Dispatch-Keys mindestens einmal.
    let verarbeitet = report.processed();
    for message_type in [
        EINLESEN_CONSUMED,
        OBFUSKIERUNG_CONSUMED,
        OBFUSCATED_MESSAGE_TYPE,
        START_MESSAGE_TYPE,
        FENSTERN_CONSUMED,
        AUSWERTEN_CONSUMED,
        CONSUMED_HISTORY_MESSAGE_TYPE,
    ] {
        assert!(
            verarbeitet
                .get(message_type)
                .is_some_and(|count| *count > 0),
            "{message_type} muss im Lauf vorgekommen sein: {verarbeitet:?}"
        );
    }
    assert_eq!(
        verarbeitet.get(EINLESEN_CONSUMED),
        Some(&1),
        "der Import wird genau einmal angestoßen"
    );
    assert_eq!(
        verarbeitet.get(START_MESSAGE_TYPE),
        Some(&1),
        "die Analyse wird genau einmal angestoßen — nicht je Buchung"
    );
    assert_eq!(
        verarbeitet.get(OBFUSKIERUNG_CONSUMED),
        verarbeitet.get(OBFUSCATED_MESSAGE_TYPE),
        "jede obfuskierte Buchung wird auch abgeräumt, sonst bliebe sie ewig offen"
    );
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    clippy::unwrap_in_result,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert                   trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn das_lauf_protokoll_traegt_keinen_rohwert_keinen_freitext_keine_iban_und_nicht_den_seed()
-> rusqlite::Result<()> {
    let db = TempDbPath::new("lauf_protokoll");
    let paths = lauf_pfade(&db);
    let mut protokoll = Vec::new();

    run(&paths, &mut protokoll).expect("der Lauf gelingt");

    let text = String::from_utf8(protokoll).expect("das Protokoll ist Text");
    for rohwert in ROHWERTE {
        assert!(
            !text.contains(rohwert),
            "das Lauf-Protokoll darf `{rohwert}` nicht enthalten:\n{text}"
        );
    }
    // Positiv: es trägt die Nachrichtentypen und die Schlussbilanz — sonst bliebe die Prüfung oben
    // auch dann grün, wenn gar nichts geschrieben würde.
    assert!(text.contains(EINLESEN_CONSUMED));
    assert!(text.contains(CONSUMED_HISTORY_MESSAGE_TYPE));
    assert!(text.contains("gesamt:"));
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert                   trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_terminal_gescheiterter_lauf_nennt_den_redigierten_fehler() -> rusqlite::Result<()> {
    // Terminal scheitern lassen, wo es real passiert: die Quelldatei fehlt. Der Fehler ist der
    // **nackte, redigierte** Domänen-Fehler des `einlesen`-Slice, den `io` durchreicht.
    let db = TempDbPath::new("lauf_terminal");
    let mut paths = lauf_pfade(&db);
    paths.transactions = fixture("einlesen/csv/gibt-es-nicht.csv");
    let mut protokoll = Vec::new();

    let fehler = run(&paths, &mut protokoll).expect_err("eine fehlende Quelldatei ist terminal");

    assert!(
        matches!(&fehler, RunError::Message { message_type, .. }
            if message_type == EINLESEN_CONSUMED),
        "der Abbruch muss die gescheiterte Nachricht benennen: {fehler:?}"
    );
    let meldung = fehler.to_string();
    assert!(
        meldung.contains("source unavailable"),
        "die durchgereichte Diagnose muss erhalten bleiben: {meldung}"
    );
    for rohwert in ROHWERTE {
        assert!(
            !meldung.contains(rohwert),
            "der Fehler darf `{rohwert}` nicht zitieren: {meldung}"
        );
    }
    // Die Nachricht bleibt **offen** — der nächste Lauf nimmt sie wieder auf (ADR-0012/Issue 0085).
    assert_eq!(offene_nachrichten(&paths)?, 1);
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    clippy::unwrap_in_result,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_lauf_schwaerzt_namen_iban_und_bic_im_verwendungszweck_bevor_sie_obfuskiert_liegen()
-> rusqlite::Result<()> {
    // Die Scrubbing-Fixture: „LISA BEISPIEL" ist Empfänger einer Buchung (also aus der Datenbank
    // bekannt), „Oma Fiktiv" steht nur in der Namensliste, und ein Verwendungszweck nennt beide
    // samt einer IBAN und einer BIC. Nichts davon darf im obfuskierten Bestand roh stehen.
    let db = TempDbPath::new("lauf_scrubbing");
    let mut paths = lauf_pfade(&db);
    paths.transactions = fixture("einlesen/csv/scrubbing.csv");
    let mut protokoll = Vec::new();

    run(&paths, &mut protokoll).expect("der Lauf über die Scrubbing-Fixture gelingt");

    let conn = rusqlite::Connection::open(&paths.database)?;
    let mut statement = conn.prepare(
        "SELECT coalesce(purpose, '') || ' ' || coalesce(recipient_name, '') FROM obfuskiert;",
    )?;
    let freitext = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let alles = freitext.join("\n");
    for rohwert in ["lisa beispiel", "oma fiktiv", "de00 1234", "fiktdeff001"] {
        assert!(
            !alles.to_lowercase().contains(rohwert),
            "der obfuskierte Freitext darf `{rohwert}` nicht roh tragen:\n{alles}"
        );
    }
    assert!(
        alles.contains("Auslage fuer [PERSON] und [PERSON] IBAN [IBAN] BIC [BIC]"),
        "Namen aus Datenbank und Namensliste, IBAN und BIC werden ersetzt:\n{alles}"
    );
    // Die Firma mit Gläubiger-ID ist kein Personen-Name: ihr Name bleibt Kategorie-Signal.
    assert!(alles.contains("STROMWERK FIKTIV AG"), "{alles}");
    Ok(())
}

#[test]
fn eine_fehlende_namensliste_beendet_den_lauf_bevor_irgendetwas_laeuft() {
    // Die Namensliste ist Pflicht: fehlt sie, wird nicht still mit weniger Namen gescrubbt.
    let db = TempDbPath::new("lauf_namen");
    let mut paths = lauf_pfade(&db);
    paths.persons_config = fixture("lauf/gibt-es-nicht.toml");
    let mut protokoll = Vec::new();

    let fehler = run(&paths, &mut protokoll).expect_err("fehlende Namensliste");

    assert_eq!(
        fehler,
        RunError::Config(ConfigError::Unreadable(crate::config::ConfigKind::Personen))
    );
    assert!(
        protokoll.is_empty(),
        "vor der geladenen Config läuft nichts"
    );
}

#[test]
fn eine_fehlende_config_beendet_den_lauf_bevor_irgendetwas_laeuft() {
    let db = TempDbPath::new("lauf_config");
    let mut paths = lauf_pfade(&db);
    paths.own_accounts_config = fixture("lauf/gibt-es-nicht.toml");
    let mut protokoll = Vec::new();

    let fehler = run(&paths, &mut protokoll).expect_err("fehlende Config");

    assert_eq!(
        fehler,
        RunError::Config(ConfigError::Unreadable(
            crate::config::ConfigKind::EigeneKonten
        ))
    );
    assert!(
        protokoll.is_empty(),
        "vor der geladenen Config wird nichts protokolliert — und nichts angestoßen"
    );
}

// --- Mehrere Läufe in eine Datenbank (Issue 0088, ADR-0016) ---------------------------------

/// Der Auszug des zweiten eigenen Kontos (Haushalt): erfunden, mit zwei gleichen Buchungen am
/// selben Tag ohne Kundenreferenz und einer vorgemerkten Zeile.
const ZWEITES_KONTO: &str = "einlesen/csv/lauf_haushalt.csv";

/// Wie viele **gebuchte** Zeilen die beiden Auszüge tragen: `lauf.csv` 53, der Haushalts-Auszug 8
/// (seine vorgemerkte Zeile zählt nicht).
const ZEILEN_ERSTES_KONTO: i64 = 53;
const ZEILEN_ZWEITES_KONTO: i64 = 8;

/// Die Pfade für einen Lauf über den Auszug `csv`, sonst wie [`lauf_pfade`].
fn lauf_pfade_mit(db: &TempDbPath, csv: &str) -> RunPaths {
    RunPaths {
        transactions: fixture(csv),
        ..lauf_pfade(db)
    }
}

/// Fährt einen Lauf und gibt seinen Bericht zurück — ein gescheiterter Lauf ist hier ein
/// Testfehler, kein Ergebnis.
fn lauf(paths: &RunPaths) -> RunReport {
    run(paths, &mut Vec::new()).expect("der Lauf gelingt")
}

/// Eine Zeile Monats-Zahlen: Jahr, Monat, ein Schlüssel (Rolle bzw. Konto-Token, sonst leer),
/// Eingang und Ausgang in Minor Units.
type Monatszahl = (i64, i64, String, i64, i64);

/// **Die Zahlen des jüngsten Laufs** — das, was eine Auswertung zeigt: wie viele Buchungen seine
/// Fenster tragen, der Cashflow je Monat, die Umbuchungen je Monat und Rolle, die Gebühren je Monat
/// und Konto, und die Zahl der Vorkommen der fenster-übergreifenden Reichweite. Gescopt über die
/// `run_id` des zuletzt abgelegten Fensters: genau das, was „der jüngste Lauf" heißt.
#[derive(Debug, PartialEq, Eq)]
struct Zahlen {
    fenster_buchungen: i64,
    monate: Vec<Monatszahl>,
    umbuchungen: Vec<Monatszahl>,
    gebuehren: Vec<Monatszahl>,
    vorkommen: usize,
}

fn zahlen_des_juengsten_laufs(paths: &RunPaths, report: &RunReport) -> rusqlite::Result<Zahlen> {
    let conn = rusqlite::Connection::open(&paths.database)?;
    let lauf: String = conn.query_row(
        "SELECT run_id FROM auswertungsfenster ORDER BY rowid DESC LIMIT 1;",
        [],
        |row| row.get(0),
    )?;
    let fenster_buchungen = conn.query_row(
        "SELECT count(*) FROM auswertungsfenster_buchungen b \
         JOIN auswertungsfenster w ON w.id = b.window_id WHERE w.run_id = ?1;",
        [&lauf],
        |row| row.get(0),
    )?;
    let monatszahlen = |tabelle: &str, schluessel: &str| -> rusqlite::Result<Vec<Monatszahl>> {
        let mut statement = conn.prepare(&format!(
            "SELECT d.year, d.month, {schluessel}, \
                    sum(d.incoming_minor_units), sum(d.outgoing_minor_units) \
             FROM {tabelle} d \
             JOIN window_evaluations e ON e.id = d.evaluation_id \
             JOIN auswertungsfenster w ON w.id = e.window_id \
             WHERE w.run_id = ?1 GROUP BY 1, 2, 3 ORDER BY 1, 2, 3;"
        ))?;
        let zeilen = statement.query_map([&lauf], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
        zeilen.collect()
    };
    Ok(Zahlen {
        fenster_buchungen,
        monate: monatszahlen("window_evaluation_months", "''")?,
        umbuchungen: monatszahlen("window_evaluation_transfers", "d.role")?,
        gebuehren: monatszahlen("window_evaluation_fees", "d.account")?,
        vorkommen: report.occurrences(),
    })
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn a_dann_b_dann_a_erneut_ergibt_dieselben_zahlen_wie_a_und_b_je_einmal() -> rusqlite::Result<()> {
    // Zwei eigene Konten, je ein Auszug, eine Datenbank — und der erste Auszug versehentlich ein
    // zweites Mal eingelesen. Ein neuer Import ergänzt, er ersetzt nicht und verdoppelt nicht: die
    // Zahlen des jüngsten Laufs müssen dieselben sein wie nach A und B je einmal.
    let db_wiederholt = TempDbPath::new("lauf_a_b_a");
    let a = lauf_pfade(&db_wiederholt);
    let b = lauf_pfade_mit(&db_wiederholt, ZWEITES_KONTO);
    lauf(&a);
    lauf(&b);
    let bericht_wiederholt = lauf(&a);

    let db_einmal = TempDbPath::new("lauf_a_b");
    let a_einmal = lauf_pfade(&db_einmal);
    lauf(&a_einmal);
    let bericht_einmal = lauf(&lauf_pfade_mit(&db_einmal, ZWEITES_KONTO));

    let wiederholt = zahlen_des_juengsten_laufs(&a, &bericht_wiederholt)?;
    let einmal = zahlen_des_juengsten_laufs(&a_einmal, &bericht_einmal)?;
    assert_eq!(wiederholt, einmal);
    // Und die Zahlen sind die **beider** Konten, nicht nur des zuletzt eingelesenen: jede Buchung
    // beider Auszüge genau einmal im Bestand und genau einmal in den Fenstern des jüngsten Laufs.
    let beide = ZEILEN_ERSTES_KONTO + ZEILEN_ZWEITES_KONTO;
    assert_eq!(zeilen(&a, "einlesen_lines")?, beide);
    assert_eq!(zeilen(&a, "obfuskiert")?, beide);
    assert_eq!(wiederholt.fenster_buchungen, beide);
    assert!(!wiederholt.monate.is_empty() && !wiederholt.umbuchungen.is_empty());
    assert_eq!(offene_nachrichten(&a)?, 0);
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_zweiter_lauf_auf_derselben_datei_ergaenzt_nichts_und_publiziert_nichts()
-> rusqlite::Result<()> {
    // Der Lauf ist resümierbar **und** wiederholbar: ein zweiter Lauf über denselben Auszug erkennt
    // jede Zeile an ihrem Zeilen-Schlüssel wieder, legt keine an und schickt keine stromabwärts.
    let db = TempDbPath::new("lauf_zweimal");
    let paths = lauf_pfade(&db);
    lauf(&paths);

    let zweiter = lauf(&paths);

    assert_eq!(zeilen(&paths, "einlesen_lines")?, ZEILEN_ERSTES_KONTO);
    assert_eq!(zeilen(&paths, "obfuskiert")?, ZEILEN_ERSTES_KONTO);
    assert_eq!(
        zweiter.processed().get(EINLESEN_CONSUMED),
        Some(&1),
        "der Import wird angestoßen …"
    );
    assert_eq!(
        zweiter.processed().get(OBFUSKIERUNG_CONSUMED),
        None,
        "… aber keine Zeile reist ein zweites Mal stromabwärts"
    );
    assert_eq!(offene_nachrichten(&paths)?, 0, "und beide laufen leer");
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_zweiter_lauf_zaehlt_keine_fenster_eines_frueheren_laufs_mit() -> rusqlite::Result<()> {
    // Jeder Lauf paginiert die **ganze** obfuskierte Historie neu — das ist gewollt, so zeigt der
    // jüngste alle Konten gemeinsam. Gemessen vor ADR-0016: 53 → 159 → 318 Fenster-Buchungen über
    // drei Läufe, weil die fenster-übergreifende Abfrage nur über das Datum scopte. Jetzt gehört
    // jedes Fenster genau einem Lauf, und der jüngste sieht nur seine.
    let db = TempDbPath::new("lauf_fenster_je_lauf");
    let paths = lauf_pfade(&db);
    let erster = lauf(&paths);
    let fenster_eines_laufs = zeilen(&paths, "auswertungsfenster")?;

    let zweiter = lauf(&paths);
    let dritter = lauf(&paths);

    let zahlen = zahlen_des_juengsten_laufs(&paths, &dritter)?;
    assert_eq!(zahlen.fenster_buchungen, ZEILEN_ERSTES_KONTO);
    assert_eq!(zweiter.occurrences(), erster.occurrences());
    assert_eq!(dritter.occurrences(), erster.occurrences());
    assert_eq!(
        zaehle(
            &paths,
            "SELECT count(DISTINCT run_id) FROM auswertungsfenster;"
        )?,
        3,
        "drei Läufe, drei Lauf-Identitäten"
    );
    assert_eq!(
        zeilen(&paths, "auswertungsfenster")?,
        3 * fenster_eines_laufs,
        "je Lauf derselbe Satz Fenster — keiner wächst mit den früheren"
    );
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn zwei_gleiche_buchungen_zaehlen_beide_und_eine_vorgemerkte_zeile_bricht_nichts()
-> rusqlite::Result<()> {
    // Der Haushalts-Auszug trägt zweimal dasselbe Brötchen am selben Tag (ohne Kundenreferenz) und
    // eine vorgemerkte Tankung mit leerer Valuta. Der Lauf geht durch; beide Brötchen zählen, die
    // Tankung nicht.
    let db = TempDbPath::new("lauf_haushalt");
    let paths = lauf_pfade_mit(&db, ZWEITES_KONTO);

    lauf(&paths);

    assert_eq!(zeilen(&paths, "einlesen_lines")?, ZEILEN_ZWEITES_KONTO);
    assert_eq!(
        zaehle(
            &paths,
            "SELECT count(*) FROM einlesen_lines WHERE purpose = 'Baeckerei Fiktiv Filiale 3';"
        )?,
        2,
        "beide gleichen Buchungen liegen im Bestand"
    );
    assert_eq!(
        zaehle(
            &paths,
            "SELECT count(*) FROM einlesen_lines WHERE purpose LIKE 'Tankstelle%';"
        )?,
        0,
        "die vorgemerkte Zeile zählt nicht"
    );
    assert_eq!(zeilen(&paths, "obfuskiert")?, ZEILEN_ZWEITES_KONTO);
    assert_eq!(offene_nachrichten(&paths)?, 0);
    Ok(())
}

/// Ein Pragma-Wert der Lauf-Datei, über eine **eigene** Verbindung gelesen.
fn pragma(paths: &RunPaths, name: &str) -> rusqlite::Result<i64> {
    rusqlite::Connection::open(&paths.database)?.pragma_query_value(None, name, |row| row.get(0))
}

#[expect(
    clippy::panic_in_result_fn,
    clippy::unwrap_in_result,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn eine_frische_datenbank_traegt_die_schema_version_und_der_lauf_geht_durch() -> rusqlite::Result<()>
{
    let db = TempDbPath::new("lauf_schema_frisch");
    let paths = lauf_pfade(&db);
    let mut protokoll = Vec::new();

    run(&paths, &mut protokoll).expect("der Lauf auf frischer Datei gelingt");

    assert_eq!(
        pragma(&paths, "user_version")?,
        schema::SCHEMA_VERSION,
        "die frische Datei trägt die Schema-Version des Programms"
    );
    assert_eq!(offene_nachrichten(&paths)?, 0);
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    clippy::unwrap_in_result,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_zweiter_lauf_gegen_dieselbe_datei_legt_nichts_neu_an_und_laeuft_durch()
-> rusqlite::Result<()> {
    let db = TempDbPath::new("lauf_schema_wieder");
    let paths = lauf_pfade(&db);
    let mut protokoll = Vec::new();
    run(&paths, &mut protokoll).expect("erster Lauf");
    // SQLites eigener Schema-Zähler wächst mit **jeder** Schema-Änderung — bleibt er stehen, wurde
    // nichts angelegt.
    let schema_vorher = pragma(&paths, "schema_version")?;

    run(&paths, &mut protokoll).expect("zweiter Lauf auf derselben Datei");

    assert_eq!(
        pragma(&paths, "schema_version")?,
        schema_vorher,
        "der zweite Lauf darf das Schema nicht anfassen"
    );
    assert_eq!(pragma(&paths, "user_version")?, schema::SCHEMA_VERSION);
    assert_eq!(offene_nachrichten(&paths)?, 0);
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn eine_datei_mit_abweichender_schema_version_laesst_den_lauf_terminal_und_redigiert_scheitern()
-> rusqlite::Result<()> {
    // Eine Datei aus der Zeit **vor** dem Stempel: sie trägt Tabellen samt Rohwerten, aber keinen
    // Versionsstempel (also Version 0). Genau so sähe eine echte Alt-Datei aus.
    let db = TempDbPath::new("lauf_schema_alt");
    let paths = lauf_pfade(&db);
    rusqlite::Connection::open(&paths.database)?.execute_batch(&format!(
        "CREATE TABLE einlesen_lines (iban TEXT, name TEXT, zweck TEXT);
         INSERT INTO einlesen_lines VALUES ('{}', '{}', '{}');",
        ROHWERTE[0], ROHWERTE[2], ROHWERTE[3]
    ))?;
    let schema_vorher = pragma(&paths, "schema_version")?;
    let mut protokoll = Vec::new();

    let fehler = run(&paths, &mut protokoll).expect_err("eine Alt-Datei ist terminal");

    assert!(
        matches!(&fehler, RunError::SchemaVersion(mismatch)
            if mismatch.found() == 0 && mismatch.expected() == schema::SCHEMA_VERSION),
        "der Abbruch muss die beiden Versionen nennen: {fehler:?}"
    );
    // Redigiert: kein Rohwert der Datei, und auch nicht ihr Pfad oder Dateiname.
    let meldung = fehler.to_string();
    let dateiname = paths
        .database
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    for verboten in ROHWERTE
        .iter()
        .map(ToString::to_string)
        .chain([paths.database.to_string_lossy().into_owned(), dateiname])
    {
        assert!(
            !meldung.contains(&verboten),
            "die Meldung darf `{verboten}` nicht enthalten: {meldung}"
        );
    }
    assert!(meldung.contains("Schema-Version 0"), "{meldung}");
    // Terminal **vor** dem ersten Schreiben: kein Stempel, keine Tabelle, nichts protokolliert.
    assert_eq!(pragma(&paths, "user_version")?, 0);
    assert_eq!(pragma(&paths, "schema_version")?, schema_vorher);
    assert!(
        protokoll.is_empty(),
        "vor dem Abbruch wird nichts angestoßen"
    );
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert                   trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_offener_ausloeser_einer_anderen_quelle_bricht_den_lauf_ab() -> rusqlite::Result<()> {
    // Der Fall, der vorher **still** danebenging: ein gescheiterter Lauf lässt seinen Auslöser
    // offen liegen. Da der Slice seine Quelle über den Datenquellen-Port *dieses* Laufs beschafft
    // (Issue 0018) und nicht über den `partition_key`, las die liegengebliebene Nachricht beim
    // nächsten Mal die **neue** Datei — der Lauf importierte sie zweimal und meldete Exit 0.
    let db = TempDbPath::new("lauf_fremder_rest");
    let mut kaputt = lauf_pfade(&db);
    kaputt.transactions = fixture("einlesen/csv/pflichtfeld_fehlt.csv");
    let mut protokoll = Vec::new();

    run(&kaputt, &mut protokoll).expect_err("ein fehlendes Pflichtfeld ist terminal");
    assert_eq!(
        offene_nachrichten(&kaputt)?,
        1,
        "der gescheiterte Auslöser bleibt offen (ADR-0012)"
    );

    let gut = lauf_pfade(&db);
    let fehler = run(&gut, &mut protokoll).expect_err("der fremde Rest muss den Lauf stoppen");

    assert!(
        matches!(&fehler, RunError::PendingImport { pending, requested }
            if pending == "pflichtfeld_fehlt.csv" && requested == "lauf.csv"),
        "der Abbruch muss beide Quellen benennen: {fehler:?}"
    );
    assert_eq!(
        zeilen(&gut, "einlesen_lines")?,
        0,
        "und zwar **bevor** irgendetwas geschrieben wurde"
    );
    Ok(())
}
