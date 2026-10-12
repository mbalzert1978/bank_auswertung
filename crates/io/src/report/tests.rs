//! Der Bericht aus **reinen Fixture-DTOs** — ohne SQLite, ohne Kategorisierer. Alle Daten sind
//! erfunden; Token, Gläubiger-IDs und Pseudonyme haben nur die Form der echten.

use std::collections::BTreeMap;

use auswerten_adapter::{
    AccountCashflowRow, CategoryAnswerRow, CostDriverRow, DriverRankRow, FeeRow,
    HistoryEvaluationRow, InternalTransferRow, PriceIncreaseRow, RecurrenceRhythm, RecurrenceRow,
    SavingsCandidateRow,
};

use super::file;
use super::html::escape;
use super::model::{Bericht, ReportInputs, UNKNOWN_CATEGORY};
use crate::pipeline_test_support::{TempDbPath, stamp};

const GIRO: &str = "KONTO_a1b2c3d4";
const OHNE_ROLLE: &str = "KONTO_c9d0e1f2";
const STROM: &str = "COMPANY:DE00ZZZ00000000001";
const BUCHLADEN: &str = "COMPANY:DE00ZZZ00000000004|MERCHANT:BUCHLADEN-FIKTIV";
const PERSON: &str = "PERSON:PERSON_3f2a1b9c8d7e";

fn art(fingerprint: &str) -> CostDriverRow {
    CostDriverRow::Kind {
        fingerprint: fingerprint.to_string(),
    }
}

fn platz(driver: CostDriverRow, outflow: u64, share: u32, cumulative: u32) -> DriverRankRow {
    DriverRankRow {
        driver,
        outflow_minor_units: outflow,
        bookings: 3,
        share_basis_points: share,
        cumulative_share_basis_points: cumulative,
        annual_amount_minor_units: outflow,
        rhythm: None,
        price_increase: None,
    }
}

fn strom() -> DriverRankRow {
    DriverRankRow {
        annual_amount_minor_units: 107_400,
        rhythm: Some(RecurrenceRhythm::Monthly),
        ..platz(art(STROM), 268_500, 6_250, 6_250)
    }
}

/// Ein kleiner, vollständiger Lauf als Fixture-DTOs: zwei Konten mit Rolle, eines ohne, eine Firma,
/// ein Händler hinter einem Zahlungsdienst, eine Person, Gebühren und eine Umbuchung.
fn eingaben() -> ReportInputs {
    ReportInputs {
        incomplete: 0,
        history: HistoryEvaluationRow {
            currency: Some("EUR".to_string()),
            occurrences: Vec::new(),
            recurrences: vec![RecurrenceRow {
                driver: art(STROM),
                rhythm: RecurrenceRhythm::Monthly,
                typical_amount_minor_units: 8_950,
                annual_amount_minor_units: 107_400,
                price_increase: Some(PriceIncreaseRow {
                    since_day: stamp(2026, 5, 5),
                    before_minor_units: 8_500,
                    after_minor_units: 8_950,
                }),
                charged_account: GIRO.to_string(),
            }],
            account_cashflows: vec![
                AccountCashflowRow {
                    account: OHNE_ROLLE.to_string(),
                    role: None,
                    year: 2026,
                    month: 4,
                    incoming_minor_units: 0,
                    outgoing_minor_units: 1_000,
                },
                AccountCashflowRow {
                    account: GIRO.to_string(),
                    role: Some("giro".to_string()),
                    year: 2026,
                    month: 3,
                    incoming_minor_units: 245_000,
                    outgoing_minor_units: 123_456,
                },
            ],
            transfers: vec![InternalTransferRow {
                year: 2026,
                month: 3,
                role: "spar".to_string(),
                incoming_minor_units: 0,
                outgoing_minor_units: 30_000,
            }],
            fees: vec![FeeRow {
                year: 2026,
                month: 3,
                account: GIRO.to_string(),
                incoming_minor_units: 0,
                outgoing_minor_units: 490,
            }],
            ranking: vec![
                strom(),
                platz(
                    CostDriverRow::Fee {
                        account: GIRO.to_string(),
                    },
                    1_470,
                    2_500,
                    8_750,
                ),
                platz(art(BUCHLADEN), 3_499, 1_000, 9_750),
                platz(art(PERSON), 7_500, 200, 9_950),
                platz(CostDriverRow::Unattributed, 100, 50, 10_000),
            ],
            savings_candidates: vec![SavingsCandidateRow {
                pareto_rank: 1,
                ranked: strom(),
            }],
        },
        categories: Vec::new(),
        company_names: BTreeMap::from([
            (
                "DE00ZZZ00000000001".to_string(),
                "STROMWERK NORD AG".to_string(),
            ),
            (
                "DE00ZZZ00000000004".to_string(),
                "ZAHLDIENST TREUHAND GMBH".to_string(),
            ),
        ]),
        persons: vec!["Erika Beispiel".to_string()],
    }
}

fn mit_kategorie(fingerprint: &str, category: &str) -> ReportInputs {
    ReportInputs {
        categories: vec![CategoryAnswerRow {
            fingerprint: fingerprint.to_string(),
            category: category.to_string(),
        }],
        ..eingaben()
    }
}

#[test]
fn der_bericht_entsteht_aus_reinen_fixture_dtos_mit_allen_abschnitten() {
    let html = Bericht::compose(&eingaben()).render();

    assert!(html.starts_with("<!DOCTYPE html>"));
    for abschnitt in [
        "Cashflow je Konto und Monat",
        "Umbuchungen zwischen eigenen Konten",
        "Gebühren",
        "Pareto-Ranking der Kostentreiber",
        "Spar-Kandidaten",
        "Wiederkehrende Abflüsse",
    ] {
        assert!(
            html.contains(&format!("<h2>{abschnitt}</h2>")),
            "{abschnitt}"
        );
    }
    // Eigenständig: keine externe Ressource, kein Skript.
    assert!(!html.contains("<script"));
    assert!(!html.contains("http"));
}

#[test]
fn der_bericht_zeigt_die_errechneten_werte_in_euro_monaten_und_prozent() {
    let bericht = Bericht::compose(&eingaben());

    // Der Cashflow trägt die Summen der Domäne, nach Konto-Beschriftung geordnet.
    let giro = bericht
        .cashflows
        .iter()
        .find(|line| line.account == "Giro")
        .expect("das Girokonto steht im Cashflow");
    assert_eq!(giro.month, "März 2026");
    assert_eq!(giro.incoming, "2.450,00 €");
    assert_eq!(giro.outgoing, "1.234,56 €");

    // Das Ranking steht in der Ordnung der Domäne, mit ihren Anteilen.
    let treiber = bericht
        .ranking
        .iter()
        .map(|line| {
            (
                line.rank.as_str(),
                line.driver.as_str(),
                line.share.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        treiber,
        vec![
            ("1", "STROMWERK NORD AG", "62,50 %"),
            ("2", "Bankgebühren Giro", "25,00 %"),
            (
                "3",
                "ZAHLDIENST TREUHAND GMBH › BUCHLADEN-FIKTIV",
                "10,00 %"
            ),
            ("4", "Person 3f2a1b", "2,00 %"),
            ("5", "Abflüsse ohne Gegenpartei", "0,50 %"),
        ]
    );
    assert_eq!(
        bericht
            .ranking
            .last()
            .map(|line| line.cumulative_share.as_str()),
        Some("100,00 %")
    );

    // Spar-Kandidat, Wiederkehr samt Preissteigerung, Umbuchung und Gebühr.
    assert_eq!(
        bericht
            .savings
            .first()
            .map(|line| (line.rank.as_str(), line.annual.as_str())),
        Some(("1", "1.074,00 €"))
    );
    let wiederkehr = bericht.recurrences.first().expect("eine Wiederkehr");
    assert_eq!(wiederkehr.rhythm, "monatlich");
    assert_eq!(
        wiederkehr.price_increase,
        "seit 05.05.2026: 85,00 € → 89,50 €"
    );
    assert_eq!(wiederkehr.account, "Giro");
    let umbuchung = bericht.transfers.first().expect("eine Umbuchung");
    assert_eq!(
        (
            umbuchung.counter_account.as_str(),
            umbuchung.outgoing.as_str()
        ),
        ("Spar", "300,00 €")
    );
    let gebuehr = bericht.fees.first().expect("eine Gebühr");
    assert_eq!(
        (gebuehr.account.as_str(), gebuehr.charged.as_str()),
        ("Giro", "4,90 €")
    );
}

#[test]
fn die_kategorie_aendert_keine_zahl_sie_ist_nur_ein_attribut_des_treibers() {
    let ohne = Bericht::compose(&eingaben());
    let mit = Bericht::compose(&mit_kategorie(STROM, "Energie"));

    // Jede Zahl ist dieselbe — sie kommt aus der Analyse, nicht aus dem Etikett.
    let zahlen = |bericht: &Bericht| {
        bericht
            .ranking
            .iter()
            .map(|line| {
                (
                    line.driver.clone(),
                    line.outflow.clone(),
                    line.share.clone(),
                    line.annual.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(zahlen(&ohne), zahlen(&mit));
    assert_eq!(ohne.cashflows, mit.cashflows);

    // Das Etikett steht genau in den Kategorie-Zellen des einen Treibers: Ranking, Spar-Kandidat,
    // Wiederkehr — und sonst nirgends.
    let html = mit.render();
    assert_eq!(html.matches("Energie").count(), 3);
    assert_eq!(
        mit.ranking.first().map(|line| line.category.as_str()),
        Some("Energie")
    );
}

#[test]
fn ohne_eine_einzige_kategorie_steht_jeder_treiber_als_unbekannt_da() {
    let bericht = Bericht::compose(&eingaben());

    let kategorien = bericht
        .ranking
        .iter()
        .map(|line| line.category.as_str())
        .chain(bericht.savings.iter().map(|line| line.category.as_str()))
        .chain(
            bericht
                .recurrences
                .iter()
                .map(|line| line.category.as_str()),
        )
        .collect::<Vec<_>>();
    assert_eq!(kategorien.len(), 7);
    assert!(
        kategorien
            .iter()
            .all(|category| *category == UNKNOWN_CATEGORY)
    );
    // Nichts geht verloren: jeder Treiber, jeder Kandidat, jede Wiederkehr steht da.
    assert_eq!(bericht.ranking.len(), 5);
    assert_eq!(bericht.savings.len(), 1);
    assert_eq!(bericht.recurrences.len(), 1);
}

#[test]
fn fehlen_der_analyse_buchungen_nennt_der_bericht_ihre_zahl_ganz_oben() {
    let html = Bericht::compose(&ReportInputs {
        incomplete: 3,
        ..eingaben()
    })
    .render();

    let hinweis = html
        .find("Analyse unvollständig")
        .expect("der Hinweis steht im Bericht");
    let erster_abschnitt = html.find("<section").expect("ein Abschnitt");
    assert!(
        hinweis < erster_abschnitt,
        "der Hinweis steht vor den Zahlen"
    );
    assert!(html.contains("3 Buchungen fehlen"));
    let eine = Bericht::compose(&ReportInputs {
        incomplete: 1,
        ..eingaben()
    })
    .render();
    assert!(eine.contains("1 Buchung fehlt."));

    let vollstaendig = Bericht::compose(&eingaben()).render();
    assert!(!vollstaendig.contains("Analyse unvollständig"));
}

#[test]
fn ein_etikett_mit_markup_erscheint_escaped() {
    let html = Bericht::compose(&mit_kategorie(
        STROM,
        "<script>alert('x')</script> & \"Co\"",
    ))
    .render();

    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt; &amp; &quot;Co&quot;"));
}

#[test]
fn jeder_text_geht_durch_das_escaping_auch_ein_firmenname() {
    let mut eingaben = eingaben();
    eingaben.company_names.insert(
        "DE00ZZZ00000000001".to_string(),
        "STROM <b>NORD</b>".to_string(),
    );

    let html = Bericht::compose(&eingaben).render();

    assert!(!html.contains("<b>NORD</b>"));
    assert!(html.contains("STROM &lt;b&gt;NORD&lt;/b&gt;"));
    assert_eq!(escape("a<b>&\"'"), "a&lt;b&gt;&amp;&quot;&#39;");
}

#[test]
fn konten_erscheinen_ueber_ihre_rolle_nie_als_token() {
    let bericht = Bericht::compose(&eingaben());
    let html = bericht.render();

    assert!(!html.contains("KONTO_"), "kein Konto-Token im Bericht");
    let konten = bericht
        .cashflows
        .iter()
        .map(|line| line.account.as_str())
        .collect::<Vec<_>>();
    // Nach Beschriftung geordnet; das Konto, das die Config nicht führt, bekommt keine erfundene
    // Rolle.
    assert_eq!(konten, vec!["Giro", "Konto ohne Rolle"]);
}

#[test]
fn zwei_konten_derselben_rolle_werden_durchnummeriert() {
    let mut eingaben = eingaben();
    eingaben
        .history
        .account_cashflows
        .iter_mut()
        .for_each(|bucket| bucket.role = Some("giro".to_string()));

    let konten = Bericht::compose(&eingaben)
        .cashflows
        .into_iter()
        .map(|line| line.account)
        .collect::<Vec<_>>();

    // Token-Ordnung: `KONTO_a1…` vor `KONTO_c9…`.
    assert_eq!(konten, vec!["Giro 1", "Giro 2"]);
}

#[test]
fn eine_leere_historie_ergibt_einen_bericht_ohne_eintraege_und_ohne_waehrung() {
    let html = Bericht::compose(&ReportInputs::default()).render();

    assert!(html.contains("Keine Einträge."));
    assert!(html.contains("Pareto-Ranking der Kostentreiber"));
    assert!(html.contains("Währung: —."));
}

#[test]
fn eine_jaehrliche_wiederkehr_steht_als_jaehrlich_und_die_teuerste_zuerst() {
    let mut eingaben = eingaben();
    eingaben.history.recurrences.insert(
        0,
        RecurrenceRow {
            driver: art(BUCHLADEN),
            rhythm: RecurrenceRhythm::Yearly,
            typical_amount_minor_units: 6_900,
            annual_amount_minor_units: 6_900,
            price_increase: None,
            charged_account: GIRO.to_string(),
        },
    );
    eingaben.history.recurrences.push(RecurrenceRow {
        driver: art(PERSON),
        rhythm: RecurrenceRhythm::Yearly,
        typical_amount_minor_units: 250_000,
        annual_amount_minor_units: 250_000,
        price_increase: None,
        charged_account: GIRO.to_string(),
    });

    let wiederkehr = Bericht::compose(&eingaben)
        .recurrences
        .into_iter()
        .map(|line| (line.driver, line.rhythm, line.annual))
        .collect::<Vec<_>>();

    assert_eq!(
        wiederkehr,
        vec![
            (
                "Person 3f2a1b".to_string(),
                "jährlich".to_string(),
                "2.500,00 €".to_string()
            ),
            (
                "STROMWERK NORD AG".to_string(),
                "monatlich".to_string(),
                "1.074,00 €".to_string()
            ),
            (
                "ZAHLDIENST TREUHAND GMBH › BUCHLADEN-FIKTIV".to_string(),
                "jährlich".to_string(),
                "69,00 €".to_string()
            ),
        ]
    );
}

#[test]
fn der_platz_im_ranking_steht_als_letzte_spalte() {
    let html = Bericht::compose(&eingaben()).render();

    assert!(html.contains("<th class=\"num\">Platz im Ranking</th></tr>"));
    assert!(!html.contains(">Platz<"));
}

#[test]
fn ein_firmenname_der_eine_person_der_namensliste_nennt_erscheint_nur_als_glaeubiger_id() {
    // Eine natürliche Person mit Gläubiger-ID (etwa eine Vermieterin, die per Lastschrift einzieht):
    // ihr Name steht als Firmenname in der Datei — der Bericht nennt ihn nicht, wenn er auf der
    // Namensliste steht, auch nicht in der Bank-Schreibweise „Nachname, Vorname".
    let mut eingaben = eingaben();
    eingaben.company_names.insert(
        "DE00ZZZ00000000001".to_string(),
        "BEISPIEL, ERIKA".to_string(),
    );

    let html = Bericht::compose(&eingaben).render();

    assert!(!html.to_lowercase().contains("erika"));
    assert!(html.contains("Gläubiger-ID DE00ZZZ00000000001"));
    // Die andere Firma behält ihren Namen.
    assert!(html.contains("ZAHLDIENST TREUHAND GMBH"));
}

#[test]
fn eine_firma_ohne_bekannten_namen_steht_ueber_ihre_glaeubiger_id_da() {
    let bericht = Bericht::compose(&ReportInputs {
        company_names: BTreeMap::new(),
        ..eingaben()
    });

    assert_eq!(
        bericht.ranking.first().map(|line| line.driver.as_str()),
        Some("Gläubiger-ID DE00ZZZ00000000001")
    );
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert \
              trägt allein die Beschaffung des Tests, nicht seinen Ausgang"
)]
#[test]
fn das_schreiben_ersetzt_das_ziel_und_laesst_keine_nachbar_datei_liegen()
-> Result<(), Box<dyn std::error::Error>> {
    let db = TempDbPath::new("bericht_datei");
    let ziel = db.report_path();
    std::fs::write(&ziel, "alter Bericht")?;

    file::replace(&ziel, "neuer Bericht")?;

    assert_eq!(std::fs::read_to_string(&ziel)?, "neuer Bericht");
    assert!(!file::neighbour(&ziel)?.exists());
    Ok(())
}

#[test]
fn ein_pfad_ohne_dateinamen_wird_abgewiesen() {
    let ergebnis = file::replace(std::path::Path::new("/"), "Bericht");

    assert_eq!(
        ergebnis.map_err(|fehler| fehler.kind()),
        Err(std::io::ErrorKind::InvalidInput)
    );
}
