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

/// Die Pfade des erfundenen Lauf-Bestands: die reale CSV-Fixture und die drei realen Configs. Die
/// **Mechanik** ist durchweg echt (SQLite-Datei, CSV-Quelle, toml-Loader) — **erfunden** sind
/// allein die Daten.
fn lauf_pfade(db: &TempDbPath) -> RunPaths {
    RunPaths {
        database: db.path().to_path_buf(),
        transactions: fixture("einlesen/csv/lauf.csv"),
        window_config: fixture("lauf/fenster.toml"),
        seed_config: fixture("lauf/seed.toml"),
        own_accounts_config: fixture("lauf/konten.toml"),
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

#[expect(
    clippy::panic_in_result_fn,
    clippy::unwrap_in_result,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert                   trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_zweiter_lauf_auf_derselben_datei_stoesst_den_import_erneut_an() -> rusqlite::Result<()> {
    // Der Lauf ist **resumierbar**, nicht idempotent: er reiht seine Auslöser jedes Mal neu ein.
    // Diese Zusage festzuhalten ist wichtiger als sie zu verstecken — wer zweimal importiert, hat
    // den Bestand zweimal. Die Entdopplung ist ein eigenes Thema (Zeilen-Schlüssel,
    // Issue 0088), kein stiller Nebeneffekt hier.
    let db = TempDbPath::new("lauf_zweimal");
    let paths = lauf_pfade(&db);
    let mut protokoll = Vec::new();

    run(&paths, &mut protokoll).expect("erster Lauf");
    let nach_dem_ersten = zeilen(&paths, "einlesen_lines")?;
    run(&paths, &mut protokoll).expect("zweiter Lauf");

    assert_eq!(
        zeilen(&paths, "einlesen_lines")?,
        nach_dem_ersten * 2,
        "zwei Läufe über dieselbe Datei importieren zweimal"
    );
    assert_eq!(offene_nachrichten(&paths)?, 0, "und beide laufen leer");
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
