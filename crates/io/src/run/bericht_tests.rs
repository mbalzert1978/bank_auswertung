//! Der **Bericht über echte Läufe** ([Issue 0012](../../../../docs/issues/0012-html-bericht.md)): echte
//! SQLite-Datei, echte CSV-Quelle, echte Loader — erfundene Daten. Belegt wird, was nur der Lauf
//! belegen kann: dass der Bericht **nur** bei Exit 0 entsteht, **genau** den eigenen Lauf zeigt, seine
//! Kategorien aus dem Register liest — und keinen Rohwert der Eingabe trägt.

use auswerten_adapter::CategoryAnswerRow;

use super::*;
use crate::pipeline_test_support::TempDbPath;
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

/// Die **Rohwerte** eines CSV-Auszugs, die nie im Bericht stehen dürfen — aus der Datei selbst
/// gelesen, damit der Test mit der Fixture mitwächst: jede IBAN (Auftragskonto und Gegenkonto), jede
/// BIC und jeder Name einer Gegenpartei **ohne** Gläubiger-ID (also einer fail-safe Person).
/// Platzhalter wie `0` (vorgemerkte Zeilen) zählen nicht.
fn rohwerte(csv: &Path) -> std::io::Result<Vec<String>> {
    const AUFTRAGSKONTO: usize = 0;
    const GLAEUBIGER_ID: usize = 5;
    const EMPFAENGER: usize = 11;
    const IBAN: usize = 12;
    const BIC: usize = 13;
    let text = std::fs::read_to_string(csv)?;
    Ok(text
        .lines()
        .skip(1)
        .flat_map(|zeile| {
            let zellen = zeile
                .split(';')
                .map(|zelle| zelle.trim_matches('"').to_string())
                .collect::<Vec<_>>();
            let zelle = |index: usize| zellen.get(index).cloned().unwrap_or_default();
            let person = zelle(GLAEUBIGER_ID).is_empty().then(|| zelle(EMPFAENGER));
            [
                Some(zelle(AUFTRAGSKONTO)),
                Some(zelle(IBAN)),
                Some(zelle(BIC)),
                person,
            ]
        })
        .flatten()
        .filter(|wert| wert.len() > 3)
        .collect())
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

    let mut verboten = rohwerte(&paths.transactions)?;
    verboten.extend(PersonenConfig::load(&paths.persons_config)?.into_names());
    verboten.push(SeedConfig::load(&paths.seed_config)?.seed().to_string());
    verboten.push("KONTO_".to_string());
    assert!(verboten.len() > 10, "die Liste prüft etwas: {verboten:?}");
    for wert in &verboten {
        assert!(!html.contains(wert.as_str()), "der Bericht trägt {wert}");
    }

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
    assert!(bericht(&zweiter)?.contains("Analyse unvollständig:</strong> 1 Buchung(en) fehlen"));
    Ok(())
}
