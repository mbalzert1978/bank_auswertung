//! Der **Aderlass über echte Läufe** ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)):
//! zwei eigene Konten, je ein Auszug, eine Datei — dann die fenster-übergreifende Reichweite über die
//! **echten** Fenster des jüngsten Laufs, mit der **echt geladenen** Eigene-Konten-Config.
//!
//! Was hier und nicht in den Specs belegt wird: dass das Token des **belasteten** Kontos (aus der
//! Obfuskierung der Auftragskonto-IBAN) dasselbe ist wie das Token, das der Loader aus derselben IBAN
//! bildet — nur dann trägt der Cashflow je Konto seine Rolle. In den Specs sind beide Token erfunden;
//! hier entstehen sie auf zwei verschiedenen Wegen. Alle Daten sind erfunden.

use std::collections::BTreeSet;

use application_kernel::{CommandDecoder, Handler};
use auswerten_adapter::{EvaluateHistoryCommandDecoder, HistoryEvaluationHandler};
use rusqlite::Connection;

use super::*;
use crate::pipeline_test_support::TempDbPath;

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
    }
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

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die Beschaffung des Tests, nicht seinen Ausgang"
)]
#[test]
fn der_cashflow_je_konto_traegt_die_rolle_aus_der_echt_geladenen_config()
-> Result<(), Box<dyn std::error::Error>> {
    let db = TempDbPath::new("aderlass_rollen");
    run(&pfade(&db, "einlesen/csv/lauf.csv"), &mut Vec::new())?;
    let paths = pfade(&db, "einlesen/csv/lauf_haushalt.csv");
    run(&paths, &mut Vec::new())?;

    // Dieselbe Verdrahtung wie im Lauf: Configs über die echten Loader, Lesen über den echten Seam.
    let seed = SeedConfig::load(&paths.seed_config)?.seed();
    let own_accounts =
        EigeneKontenConfig::load(&paths.own_accounts_config)?.to_own_accounts_config(seed);
    let threshold = SparenConfig::load(&paths.savings_config)?.savings_threshold();
    let outbox = schema::open(&AppConfig::new(&paths.database))?;
    let (lauf, von, bis) = outbox.with_connection(juengster_lauf)?;
    let quelle = AuswertungsfensterSource::new(outbox.clone());
    let kommando = EvaluateHistoryCommandDecoder::decode(&format!("{von}:{bis}:{lauf}"), "")
        .ok_or("der Abschluss des Laufs decodiert")?;

    let auswertung = HistoryEvaluationHandler::new(&quelle, own_accounts, threshold)
        .handle(kommando)
        .map_err(|fehler| fehler.to_string())?;

    // Beide Konten tragen ihre Rolle — das Girokonto des ersten, das Haushaltskonto des zweiten
    // Auszugs. Keines steht ohne Rolle da.
    let rollen = auswertung
        .account_cashflows
        .iter()
        .map(|bucket| bucket.role.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        rollen,
        BTreeSet::from([Some("giro".to_string()), Some("haushalt".to_string())])
    );
    assert_eq!(auswertung.currency.as_deref(), Some("EUR"));

    // Und Ranking und Cashflow sehen dieselben genetteten Abflüsse.
    let abfluss_cashflow = auswertung
        .account_cashflows
        .iter()
        .map(|bucket| bucket.outgoing_minor_units)
        .sum::<u64>();
    let abfluss_ranking = auswertung
        .ranking
        .iter()
        .map(|platz| platz.outflow_minor_units)
        .sum::<u64>();
    assert_eq!(abfluss_ranking, abfluss_cashflow);
    assert!(abfluss_ranking > 0);
    Ok(())
}
