//! Der **Bericht über echte Läufe** ([Issue 0012](../../../../docs/issues/0012-html-bericht.md)): echte
//! SQLite-Datei, echte CSV-Quelle, echte Loader — erfundene Daten. Belegt wird, was nur der Lauf
//! belegen kann: dass der Bericht **nur** bei Exit 0 entsteht, **genau** den eigenen Lauf zeigt, seine
//! Kategorien aus dem Register liest — und keinen Rohwert der Eingabe trägt.

use auswerten_adapter::{
    CategoryAnswerRow, CostDriverRow, DriverRankRow, HistoryEvaluationRow, KindAnchorRow,
};

use super::*;
use crate::pipeline_test_support::{TempDbPath, VerboteneWerte};
use crate::report::neighbour_of;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR")))
}

fn pfade(db: &TempDbPath, csv: &str) -> RunPaths {
    RunPaths {
        database: db.path().to_path_buf(),
        transactions: fixture(csv),
        window_config: fixture("lauf/fenster.toml"),
        seed_config: fixture("lauf/seed.toml"),
        own_accounts_config: fixture("lauf/konten.toml"),
        persons_config: fixture("lauf/namen.toml"),
        savings_config: fixture("lauf/sparen.toml"),
        category_export: None,
        report: db.report_path(),
    }
}

fn bericht(paths: &RunPaths) -> std::io::Result<String> {
    std::fs::read_to_string(&paths.report)
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die Beschaffung des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_echter_lauf_schreibt_einen_bericht_ohne_iban_bic_klarname_token_und_seed()
-> Result<(), Box<dyn std::error::Error>> {
    let db = TempDbPath::new("bericht_leak");
    // Ein **markanter** Seed, der sonst in keiner Fixture vorkommt.
    let paths = RunPaths {
        seed_config: fixture("lauf/seed_markant.toml"),
        ..pfade(&db, "einlesen/csv/lauf.csv")
    };

    run(&paths, &mut Vec::new())?;
    let html = bericht(&paths)?;

    let verboten = VerboteneWerte::aus_lauf(&paths);
    assert!(
        verboten.anzahl() > 10,
        "die Liste prüft etwas: {verboten:?}"
    );
    assert!(verboten.enthaelt("MAX MUSTERMANN") && verboten.enthaelt("MUSTERMANN"));
    verboten.pruefe("Bericht", &html);

    // Und er trägt die Analyse: Konten über ihre Rolle, Treiber, Beträge in Euro.
    for erwartet in ["Pareto-Ranking", "Giro", "Spar", "€", "unbekannt"] {
        assert!(html.contains(erwartet), "{erwartet}");
    }
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die Beschaffung des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_gescheiterter_lauf_schreibt_keinen_bericht_und_laesst_den_alten_stehen()
-> Result<(), Box<dyn std::error::Error>> {
    let db = TempDbPath::new("bericht_exit_1");
    let paths = pfade(&db, "einlesen/csv/pflichtfeld_fehlt.csv");
    std::fs::write(&paths.report, "alter Bericht")?;

    let fehler = run(&paths, &mut Vec::new()).expect_err("ein fehlendes Pflichtfeld ist terminal");

    assert!(matches!(fehler, RunError::DeadLettered(_)), "{fehler:?}");
    assert_eq!(bericht(&paths)?, "alter Bericht");
    assert!(!neighbour_of(&paths.report)?.exists());
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die Beschaffung des Tests, nicht seinen Ausgang"
)]
#[test]
fn der_bericht_zeigt_genau_den_eigenen_lauf_auch_neben_einem_frueheren()
-> Result<(), Box<dyn std::error::Error>> {
    let db = TempDbPath::new("bericht_eigener_lauf");
    let paths = pfade(&db, "einlesen/csv/lauf.csv");

    let erster = run(&paths, &mut Vec::new())?;
    let erster_bericht = bericht(&paths)?;
    let zweiter = run(&paths, &mut Vec::new())?;
    let zweiter_bericht = bericht(&paths)?;

    // Der Lauf gibt seine gemintete Identität heraus — und sie ist die jüngste der Datei.
    assert!(zweiter.run_id() > erster.run_id());
    let juengster: String =
        schema::open(&AppConfig::new(&paths.database))?.with_connection(|conn| {
            conn.query_row("SELECT max(run_id) FROM auswertungsfenster;", [], |row| {
                row.get(0)
            })
        })?;
    assert_eq!(zweiter.run_id(), juengster);
    // Beide Läufe sahen denselben Bestand. Zählte der zweite die Fenster des ersten mit, stünden
    // Umbuchungen und Gebühren doppelt da.
    assert_eq!(erster_bericht, zweiter_bericht);
    assert!(
        erster_bericht.contains("300,00 €"),
        "die Umbuchung aufs Sparkonto"
    );
    assert!(erster_bericht.contains("4,90 €"), "die Kontoführung");
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die Beschaffung des Tests, nicht seinen Ausgang"
)]
#[test]
fn die_kategorie_kommt_aus_dem_register_und_erscheint_escaped()
-> Result<(), Box<dyn std::error::Error>> {
    let db = TempDbPath::new("bericht_register");
    let paths = pfade(&db, "einlesen/csv/lauf.csv");
    run(&paths, &mut Vec::new())?;
    assert!(!bericht(&paths)?.contains("Energie"));

    // Eine Firma, die die Domäne ausgegeben hat — alle Firmen der Fixture sind Abflüsse, stehen also
    // im Ranking.
    let outbox = schema::open(&AppConfig::new(&paths.database))?;
    let firma: String = outbox.with_connection(|conn| {
        conn.query_row(
            "SELECT fingerprint FROM window_evaluation_fingerprints \
             WHERE fingerprint LIKE 'COMPANY:%' ORDER BY fingerprint LIMIT 1;",
            [],
            |row| row.get(0),
        )
    })?;
    SqliteCategoryRegistry::new(outbox).learn(&[CategoryAnswerRow {
        fingerprint: firma,
        category: "<b>Energie</b> & Wasser".to_string(),
    }])?;

    run(&paths, &mut Vec::new())?;
    let html = bericht(&paths)?;

    assert!(html.contains("&lt;b&gt;Energie&lt;/b&gt; &amp; Wasser"));
    assert!(!html.contains("<b>Energie</b>"));
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die Beschaffung des Tests, nicht seinen Ausgang"
)]
#[test]
fn ein_lauf_mit_fehlenden_buchungen_nennt_ihre_zahl_im_bericht()
-> Result<(), Box<dyn std::error::Error>> {
    // Wie im Lauf-Test zu ADR-0017: eine vergiftete `obfuskierung`-Nachricht in Lauf 1 fehlt der
    // Analyse fortan; Lauf 2 gelingt (Exit 0) und muss das im Bericht sagen.
    let db = TempDbPath::new("bericht_unvollstaendig");
    schema::open(&AppConfig::new(db.path()))?.enqueue(OBFUSKIERUNG_CONSUMED, "gibt-es-nicht")?;
    run(&pfade(&db, "einlesen/csv/lauf.csv"), &mut Vec::new())
        .expect_err("Lauf 1 hat selbst ausgesondert");

    let zweiter = pfade(&db, "einlesen/csv/lauf_haushalt.csv");
    let bilanz = run(&zweiter, &mut Vec::new())?;

    assert_eq!(bilanz.incomplete(), 1);
    assert!(bericht(&zweiter)?.contains("Analyse unvollständig:</strong> 1 Buchung fehlt."));
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die Beschaffung des Tests, nicht seinen Ausgang"
)]
#[test]
fn der_bericht_liest_die_eigene_run_id_auch_wenn_ein_juengerer_fremder_lauf_in_der_datei_liegt()
-> Result<(), Box<dyn std::error::Error>> {
    let db = TempDbPath::new("bericht_fremder_lauf");
    let paths = pfade(&db, "einlesen/csv/lauf.csv");
    let eigener = run(&paths, &mut Vec::new())?;
    let outbox = schema::open(&AppConfig::new(&paths.database))?;

    // Ein **jüngerer, fremder** Lauf: lexikografisch größer als jede Zeitstempel-Identität, mit einem
    // Fenster, dessen Buchung dieselbe Firma unter einem anderen Namen führt. Läse der Bericht
    // `max(run_id)`, stünde dieser Name darin.
    const FREMD: &str = "Z-fremder-lauf";
    let glaeubiger: String = outbox.with_connection(|conn| {
        conn.query_row(
            "SELECT recipient_creditor_id FROM obfuskiert WHERE recipient_kind = 'company' \
             ORDER BY recipient_creditor_id LIMIT 1;",
            [],
            |row| row.get(0),
        )
    })?;
    let eigener_name: String = outbox.with_connection(|conn| {
        conn.query_row(
            "SELECT recipient_name FROM obfuskiert WHERE recipient_creditor_id = ?1 LIMIT 1;",
            [&glaeubiger],
            |row| row.get(0),
        )
    })?;
    outbox.with_connection(|conn| {
        conn.execute(
            "INSERT INTO obfuskiert (id, source_id, ordering_account, amount_minor_units, currency, \
             direction, recipient_kind, recipient_name, recipient_creditor_id, booking_day, \
             value_date, seed_marker) VALUES ('fremd-buchung', 'fremd-quelle', 'KONTO_FREMD', 100, \
             'EUR', 'outgoing', 'company', 'FREMDER LAUF GMBH', ?1, '2099-12-31', '2099-12-31', \
             'fremd');",
            [&glaeubiger],
        )?;
        conn.execute(
            "INSERT INTO auswertungsfenster (id, run_id, from_day, to_day, currency) \
             VALUES ('fremd-fenster', ?1, 0, 1, 'EUR');",
            [FREMD],
        )?;
        conn.execute(
            "INSERT INTO auswertungsfenster_buchungen (window_id, position, booking_id) \
             VALUES ('fremd-fenster', 0, 'fremd-buchung');",
            [],
        )
    })?;
    let juengster: String = outbox.with_connection(|conn| {
        conn.query_row("SELECT max(run_id) FROM auswertungsfenster;", [], |row| {
            row.get(0)
        })
    })?;
    assert_eq!(juengster, FREMD);

    let bericht_von = |lauf: &str| -> Result<String, Box<dyn std::error::Error>> {
        let ziel = db.report_path();
        crate::report::write(
            &outbox,
            &ziel,
            crate::report::RunFacts {
                run_id: lauf,
                incomplete: 0,
                history: HistoryEvaluationRow {
                    ranking: vec![DriverRankRow {
                        driver: CostDriverRow::Kind {
                            fingerprint: format!("COMPANY:{glaeubiger}"),
                            anchor: KindAnchorRow::Company {
                                creditor_id: glaeubiger.clone(),
                                merchant: None,
                            },
                        },
                        outflow_minor_units: 100,
                        bookings: 1,
                        share_basis_points: 10_000,
                        cumulative_share_basis_points: 10_000,
                        annual_amount_minor_units: 100,
                        rhythm: None,
                        price_increase: None,
                    }],
                    ..HistoryEvaluationRow::default()
                },
                persons: Vec::new(),
            },
        )
        .map_err(|crate::report::ReportError(diagnose)| diagnose)?;
        Ok(std::fs::read_to_string(&ziel)?)
    };

    let eigener_bericht = bericht_von(eigener.run_id())?;
    assert!(eigener_bericht.contains(&eigener_name));
    assert!(!eigener_bericht.contains("FREMDER LAUF GMBH"));
    // Die Gegenprobe: der fremde Lauf sähe den fremden Namen — die Abfrage scopt wirklich.
    assert!(bericht_von(FREMD)?.contains("FREMDER LAUF GMBH"));
    Ok(())
}
