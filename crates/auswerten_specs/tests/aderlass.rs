//! Black-Box-Spezifikation des **Aderlasses** — Cashflow je Konto und Monat, Pareto-Ranking der
//! Kostentreiber und Spar-Kandidaten
//! ([Issue 0007](../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)) —
//! ausschließlich über die public API des Adapters (Fluent-`HistoryTestApi` + DTOs).
//!
//! Er sitzt auf der **fenster-übergreifenden** Reichweite: nur sie sieht den ganzen Lauf. Ein
//! Ranking je ~400-Tage-Seite rankte einen Ausschnitt, und ein Monat an der Seitengrenze zerfiele in
//! zwei halbe Buckets. Das Ergebnis wird **zurückgegeben**, nie abgelegt.
//!
//! Was hier festgehalten ist: alles **genettet** (eine Umbuchung aufs eigene Konto ist kein
//! Abfluss), Gebühren bleiben drin und sind eigene Treiber je Konto, Anteile in Basispunkten
//! (10 000 = 100 %), und ein Spar-Kandidat ist **jeder** Treiber, dessen **Jahresbetrag** die
//! Schwelle erreicht — ein Vorschlag, kein LLM.

use auswerten_adapter::{
    BookingDirection, CostDriverRow, DriverRankRow, EvaluationError, HistoryEvaluationRow,
    HistoryTestApi, RecurrenceRhythm, RecurrenceRow, SavingsCandidateRow, WindowBookingRow,
};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{
    APR_15, EIGENES_KONTO, FEB_15, FEB_29, HAENDLER, JAN_15, JAN_31, MAERZ_2023, MAERZ_2024,
    MAERZ_2025, MAR_15, SPARKONTO, ZWEITER_HAENDLER, ZWEITES_KONTO, art, ausgang, dauerauftrag,
    eigene_konten, eingang, entgelt, entgelt_auf, fenster, fenster_in, fingerabdruck,
    gebuehren_von, gegenkonto_eingang, historie_api, konto_monat, platz, preissteigerung, schwelle,
    vorkommen, wiederkehr, wiederkehrend, zweiter_haendler,
};

/// Die Lauf-Spanne, die alle Tage dieses Specs umfasst.
const LAUF: (i64, i64) = (MAERZ_2023, MAERZ_2025);

/// Fährt die Reichweite über die ganze Spanne.
fn auswertung(api: &HistoryTestApi) -> HistoryEvaluationRow {
    api.run_history(LAUF.0, LAUF.1)
        .expect("die fenster-übergreifende Reichweite läuft durch")
}

/// Eine TestApi über **ein** Fenster mit den gegebenen Buchungen.
fn ein_fenster(bookings: &[WindowBookingRow]) -> HistoryTestApi {
    historie_api().with_window("fenster-1", fenster(bookings))
}

// --- Cashflow je Konto und Monat -----------------------------------------------------

#[test]
fn der_cashflow_steht_je_eigenem_konto_und_monat_ueber_alle_fenster_des_laufs() {
    // Der Januar liegt in **zwei** Fenstern (der Pager hat zwischen dem 15. und dem 31. geschnitten)
    // — im Ergebnis ist er trotzdem **ein** Bucket. Das zweite Konto führt die Config nicht: es hat
    // Cashflow, aber keine Rolle.
    let api = historie_api()
        .with_own_accounts(eigene_konten(&[(EIGENES_KONTO, "giro")]))
        .with_window(
            "fenster-1",
            fenster(&[
                wiederkehrend(JAN_15, 2_000),
                eingang(JAN_15, 100_000),
                entgelt_auf(FEB_15, 500, ZWEITES_KONTO),
            ]),
        )
        .with_window(
            "fenster-2",
            fenster(&[
                ausgang(JAN_31, 3_000),
                WindowBookingRow {
                    ordering_account: ZWEITES_KONTO.to_string(),
                    ..zweiter_haendler(FEB_15, 1_000)
                },
            ]),
        );

    assert_eq!(
        auswertung(&api).account_cashflows,
        vec![
            konto_monat(EIGENES_KONTO, Some("giro"), (2024, 1), 100_000, 5_000),
            konto_monat(ZWEITES_KONTO, None, (2024, 2), 0, 1_500),
        ]
    );
}

#[test]
fn eine_genettete_umbuchung_taucht_weder_im_cashflow_noch_im_ranking_auf() {
    // Ein Dauerauftrag aufs eigene Sparkonto und ein Zufluss von dort: Geld, das zwischen den eigenen
    // Konten wandert, ist kein Aderlass.
    let buchungen = [
        dauerauftrag(JAN_15, 50_000, SPARKONTO),
        wiederkehrend(JAN_15, 1_299),
        gegenkonto_eingang(FEB_15, 20_000, SPARKONTO),
    ];
    let genettet = auswertung(
        &ein_fenster(&buchungen).with_own_accounts(eigene_konten(&[(SPARKONTO, "spar")])),
    );

    assert_eq!(
        genettet.account_cashflows,
        vec![konto_monat(EIGENES_KONTO, None, (2024, 1), 0, 1_299)]
    );
    assert_eq!(
        genettet.ranking,
        vec![platz(
            art(&fingerabdruck(HAENDLER)),
            (1_299, 1),
            (10_000, 10_000),
            1_299
        )]
    );

    // Die Gegenprobe: ist das Sparkonto **nicht** als eigenes bekannt, ist derselbe Dauerauftrag ein
    // echter Abfluss an eine Person — und der größte Treiber. Es ist das Netting, das ihn entfernt.
    let ungenettet = auswertung(&ein_fenster(&buchungen));
    assert_eq!(
        ungenettet.ranking.first().map(|platz| platz.driver.clone()),
        Some(art("PERSON:PERSON_7"))
    );
}

#[test]
fn eine_gebuehr_bleibt_im_cashflow_und_ist_ein_eigener_treiber_je_konto() {
    let api = ein_fenster(&[
        entgelt(JAN_15, 990),
        entgelt(FEB_15, 990),
        entgelt_auf(FEB_15, 500, ZWEITES_KONTO),
        wiederkehrend(JAN_15, 1_000),
    ]);

    let ergebnis = auswertung(&api);

    // Im Cashflow: die Gebühr ist echter Abfluss.
    assert_eq!(
        ergebnis.account_cashflows,
        vec![
            konto_monat(EIGENES_KONTO, None, (2024, 1), 0, 1_990),
            konto_monat(EIGENES_KONTO, None, (2024, 2), 0, 990),
            konto_monat(ZWEITES_KONTO, None, (2024, 2), 0, 500),
        ]
    );
    // Im Ranking: je belastetem Konto ein Gebühren-Treiber, neben der Buchungsart.
    assert_eq!(
        ergebnis.ranking,
        vec![
            platz(
                gebuehren_von(EIGENES_KONTO),
                (1_980, 2),
                (5_689, 5_689),
                1_980
            ),
            platz(
                art(&fingerabdruck(HAENDLER)),
                (1_000, 1),
                (2_874, 8_563),
                1_000
            ),
            platz(gebuehren_von(ZWEITES_KONTO), (500, 1), (1_437, 10_000), 500),
        ]
    );
}

// --- Pareto-Ranking --------------------------------------------------------------

#[test]
fn das_ranking_sortiert_die_treiber_nach_anteil_am_genetteten_abfluss_mit_kumuliertem_anteil() {
    let api = ein_fenster(&[
        zweiter_haendler(JAN_15, 3_000),
        wiederkehrend(JAN_15, 3_000),
        entgelt(FEB_15, 1_000),
        wiederkehrend(FEB_15, 3_000),
    ]);

    // Weniger als ein Jahr Daten: der Jahresbetrag ist schlicht die Summe (nie hochgerechnet).
    assert_eq!(
        auswertung(&api).ranking,
        vec![
            platz(
                art(&fingerabdruck(HAENDLER)),
                (6_000, 2),
                (6_000, 6_000),
                6_000
            ),
            platz(
                art(&fingerabdruck(ZWEITER_HAENDLER)),
                (3_000, 1),
                (3_000, 9_000),
                3_000
            ),
            platz(
                gebuehren_von(EIGENES_KONTO),
                (1_000, 1),
                (1_000, 10_000),
                1_000
            ),
        ]
    );
}

#[test]
fn ein_wiederkehrender_treiber_traegt_takt_und_jahresbetrag_der_wiederkehr_im_ranking() {
    let api = ein_fenster(&[
        wiederkehrend(JAN_15, 1_299),
        wiederkehrend(FEB_15, 1_299),
        wiederkehrend(MAR_15, 1_299),
        zweiter_haendler(APR_15, 10_000),
    ]);

    // Der Einzelkauf fließt im Lauf mehr ab und steht vorn; der wiederkehrende Treiber trägt seinen
    // Jahresbetrag aus der Wiederkehr (12 × 12,99 €), nicht aus dem, was im Lauf abfloss.
    assert_eq!(
        auswertung(&api).ranking,
        vec![
            platz(
                art(&fingerabdruck(ZWEITER_HAENDLER)),
                (10_000, 1),
                (7_195, 7_195),
                10_000
            ),
            DriverRankRow {
                rhythm: Some(RecurrenceRhythm::Monthly),
                ..platz(
                    art(&fingerabdruck(HAENDLER)),
                    (3_897, 3),
                    (2_805, 10_000),
                    15_588,
                )
            },
        ]
    );
}

#[test]
fn abfluesse_ohne_gegenpartei_sammeln_sich_in_einem_treiber_und_werden_nie_spar_kandidat() {
    // Zwei Abflüsse ohne Gegenpartei (etwa Barabhebungen) — sie fehlen nicht, sonst ergäben die
    // Anteile nicht das Ganze. Bei Gleichstand steht die Art vor dem Sammel-Treiber.
    let api = ein_fenster(&[
        ausgang(JAN_15, 4_000),
        ausgang(FEB_15, 1_000),
        wiederkehrend(JAN_15, 5_000),
    ])
    .with_savings_threshold(schwelle(1));

    let ergebnis = auswertung(&api);

    assert_eq!(
        ergebnis.ranking,
        vec![
            platz(
                art(&fingerabdruck(HAENDLER)),
                (5_000, 1),
                (5_000, 5_000),
                5_000
            ),
            platz(
                CostDriverRow::Unattributed,
                (5_000, 2),
                (5_000, 10_000),
                5_000
            ),
        ]
    );
    // „Spar an allem ohne Gegenpartei" ist kein Vorschlag — der Sammel-Treiber bleibt draußen.
    assert_eq!(
        ergebnis
            .savings_candidates
            .iter()
            .map(|kandidat| kandidat.ranked.driver.clone())
            .collect::<Vec<_>>(),
        vec![art(&fingerabdruck(HAENDLER))]
    );
}

#[test]
fn die_anteile_aller_plaetze_ergeben_genau_das_ganze() {
    // Drei gleich große Treiber: getrennt abgerundet wäre jeder 3 333 Basispunkte, zusammen 9 999.
    // Abgerundet wird nur der kumulierte Anteil; der letzte Platz trägt den Rest.
    let api = ein_fenster(&[
        wiederkehrend(JAN_15, 1_000),
        zweiter_haendler(JAN_15, 1_000),
        entgelt(FEB_15, 1_000),
    ]);

    let ranking = auswertung(&api).ranking;

    assert_eq!(
        ranking
            .iter()
            .map(|platz| (
                platz.share_basis_points,
                platz.cumulative_share_basis_points
            ))
            .collect::<Vec<_>>(),
        vec![(3_333, 3_333), (3_333, 6_666), (3_334, 10_000)]
    );
    assert_eq!(
        ranking
            .iter()
            .map(|platz| platz.share_basis_points)
            .sum::<u32>(),
        10_000
    );
}

#[test]
fn eine_monatliche_gebuehr_ist_wiederkehrend_mit_vollem_jahresbetrag() {
    // Die Kontoführung, Monat für Monat 4,90 € — sie ist so wiederkehrend wie ein Abo. Ihr
    // Jahresbetrag ist darum zwölf Monate (58,80 €), nicht die Summe der drei gesehenen (14,70 €).
    let api = ein_fenster(&[
        entgelt(JAN_15, 490),
        entgelt(FEB_15, 490),
        entgelt(MAR_15, 490),
    ]);

    let ergebnis = auswertung(&api);

    assert_eq!(
        ergebnis.recurrences,
        vec![RecurrenceRow {
            driver: gebuehren_von(EIGENES_KONTO),
            ..wiederkehr("", RecurrenceRhythm::Monthly, 490, 5_880)
        }]
    );
    assert_eq!(
        ergebnis.ranking,
        vec![DriverRankRow {
            rhythm: Some(RecurrenceRhythm::Monthly),
            ..platz(
                gebuehren_von(EIGENES_KONTO),
                (1_470, 3),
                (10_000, 10_000),
                5_880,
            )
        }]
    );
}

#[test]
fn eine_zweite_gebuehrenart_auf_demselben_konto_bricht_den_takt_der_kontofuehrung() {
    // Eine **bekannte Grenze**, festgehalten statt verschwiegen: `Fee(konto)` ist **eine** Reihe je
    // Konto. Neben der monatlichen Kontoführung zieht die Bank einmal ein Auslandseinsatz-Entgelt
    // ein — der Abstand von 14 Tagen fällt aus dem Takt, die Kontoführung gilt nicht mehr als
    // wiederkehrend, und ihr Jahresbetrag ist die Summe statt zwölf Monate. Ein Schnitt je
    // Gebührenart ist bewusst nicht gebaut (HITL zu Issue 0006).
    let api = ein_fenster(&[
        entgelt(JAN_15, 490),
        entgelt(FEB_15, 490),
        WindowBookingRow {
            booking_text: Some("AUSLANDSEINSATZENTGELT".to_string()),
            ..entgelt(FEB_29, 150)
        },
        entgelt(MAR_15, 490),
    ]);

    let ergebnis = auswertung(&api);

    assert_eq!(ergebnis.recurrences, vec![]);
    assert_eq!(
        ergebnis.ranking,
        vec![platz(
            gebuehren_von(EIGENES_KONTO),
            (1_620, 4),
            (10_000, 10_000),
            1_620
        )]
    );
}

#[test]
fn eine_gebuehr_mit_gegenpartei_zerreisst_die_abo_reihe_nicht() {
    // Derselbe Händler zieht einmal eine Gebühr ein (Buchungstext ENTGELT) — sie ist Geld, das als
    // Gebühr genommen wird, und gehört zu den Gebühren des Kontos. Die Abo-Reihe des Händlers bleibt
    // unberührt: 12,99 € im Monat, ohne einen Ausreißer von 3 € mitten darin.
    let api = ein_fenster(&[
        wiederkehrend(JAN_15, 1_299),
        wiederkehrend(FEB_15, 1_299),
        WindowBookingRow {
            booking_text: Some("ENTGELT RUECKLASTSCHRIFT".to_string()),
            ..wiederkehrend(FEB_29, 300)
        },
        wiederkehrend(MAR_15, 1_299),
        wiederkehrend(APR_15, 1_299),
    ]);

    let ergebnis = auswertung(&api);

    assert_eq!(
        ergebnis.recurrences,
        vec![wiederkehr(
            &fingerabdruck(HAENDLER),
            RecurrenceRhythm::Monthly,
            1_299,
            15_588
        )]
    );
    assert_eq!(
        ergebnis
            .ranking
            .iter()
            .map(|platz| (platz.driver.clone(), platz.outflow_minor_units))
            .collect::<Vec<_>>(),
        vec![
            (art(&fingerabdruck(HAENDLER)), 5_196),
            (gebuehren_von(EIGENES_KONTO), 300),
        ]
    );
}

#[test]
fn ein_lauf_mit_gemischten_waehrungen_wird_terminal_und_redigiert_abgelehnt() {
    // Jedes Fenster hat eine Währung — der Lauf darüber aber zwei. Diese Reichweite summiert über
    // alle Fenster; Euro zu Dollar zu addieren ergäbe Zahlen, die sich richtig lesen und falsch sind.
    let api = historie_api()
        .with_window("fenster-eur", fenster(&[wiederkehrend(JAN_15, 1_299)]))
        .with_window(
            "fenster-usd",
            fenster_in("USD", &[zweiter_haendler(FEB_15, 2_000)]),
        );

    let fehler = api
        .run_history(LAUF.0, LAUF.1)
        .expect_err("gemischte Währungen sind terminal");

    assert_eq!(fehler, EvaluationError::MixedCurrencies);
    let meldung = fehler.to_string();
    assert!(
        !meldung.contains("USD") && !meldung.contains("EUR"),
        "{meldung}"
    );
}

// --- Spar-Kandidaten -------------------------------------------------------------

/// Ein wiederkehrender Treiber (Jahresbetrag 15 588), ein Einzelkauf knapp darunter (15 587) und
/// eine kleine Gebühr — die Lage, an der die Schwelle trennt.
fn drei_treiber() -> HistoryTestApi {
    ein_fenster(&[
        wiederkehrend(JAN_15, 1_299),
        wiederkehrend(FEB_15, 1_299),
        wiederkehrend(MAR_15, 1_299),
        zweiter_haendler(APR_15, 15_587),
        entgelt(FEB_15, 990),
    ])
}

/// Die Treiber der Spar-Kandidaten in ihrer Reihenfolge, je mit Platz im Ranking und Jahresbetrag.
fn kandidaten(ergebnis: &HistoryEvaluationRow) -> Vec<(usize, CostDriverRow, u64)> {
    ergebnis
        .savings_candidates
        .iter()
        .map(|kandidat| {
            (
                kandidat.pareto_rank,
                kandidat.ranked.driver.clone(),
                kandidat.ranked.annual_amount_minor_units,
            )
        })
        .collect()
}

#[test]
fn spar_kandidat_ist_jeder_treiber_dessen_jahresbetrag_die_schwelle_erreicht() {
    // Die Schwelle schließt ein: 15 588 erreicht 15 588. Der Einzelkauf mit 15 587 liegt darunter —
    // obwohl er im Lauf mehr abfloss als das Abo. Bezugsgröße ist das Jahr, nicht der Lauf.
    assert_eq!(
        kandidaten(&auswertung(
            &drei_treiber().with_savings_threshold(schwelle(15_588))
        )),
        vec![(2, art(&fingerabdruck(HAENDLER)), 15_588)]
    );

    // Einen Cent tiefer sind es zwei — geordnet nach Jahresbetrag, nicht nach Platz im Ranking.
    assert_eq!(
        kandidaten(&auswertung(
            &drei_treiber().with_savings_threshold(schwelle(15_587))
        )),
        vec![
            (2, art(&fingerabdruck(HAENDLER)), 15_588),
            (1, art(&fingerabdruck(ZWEITER_HAENDLER)), 15_587),
        ]
    );
}

#[test]
fn ohne_gesetzte_schwelle_schlaegt_die_testapi_nichts_vor() {
    assert_eq!(kandidaten(&auswertung(&drei_treiber())), vec![]);
}

#[test]
fn ein_einzelkauf_zaehlt_mit_seinem_durchschnitt_je_jahr_ueber_den_beobachteten_zeitraum() {
    // Die Daten reichen vom 01.03.2023 bis zum 03.03.2025: 734 Tage. Ein Einzelkauf von 200 € darin
    // kostet im Schnitt 200 € × 365 / 734 = 99,45 € im Jahr (abgerundet auf den Cent).
    let zwei_jahre = || {
        ein_fenster(&[
            ausgang(MAERZ_2023, 100),
            zweiter_haendler(MAERZ_2024, 20_000),
            ausgang(MAERZ_2025, 100),
        ])
    };

    assert_eq!(
        kandidaten(&auswertung(
            &zwei_jahre().with_savings_threshold(schwelle(9_945))
        )),
        vec![(1, art(&fingerabdruck(ZWEITER_HAENDLER)), 9_945)]
    );
    assert_eq!(
        kandidaten(&auswertung(
            &zwei_jahre().with_savings_threshold(schwelle(9_946))
        )),
        vec![]
    );
}

#[test]
fn eine_preissteigerung_geht_als_merkmal_an_den_spar_kandidaten() {
    // Ab März kostet das Abo 14,99 € statt 12,99 € — der Jahresbetrag rechnet mit dem neuen Preis.
    let api = ein_fenster(&[
        wiederkehrend(JAN_15, 1_299),
        wiederkehrend(FEB_15, 1_299),
        wiederkehrend(MAR_15, 1_499),
        wiederkehrend(APR_15, 1_499),
    ])
    .with_savings_threshold(schwelle(10_000));

    assert_eq!(
        auswertung(&api).savings_candidates,
        vec![SavingsCandidateRow {
            pareto_rank: 1,
            ranked: DriverRankRow {
                rhythm: Some(RecurrenceRhythm::Monthly),
                price_increase: Some(preissteigerung(MAR_15, 1_299, 1_499)),
                ..platz(
                    art(&fingerabdruck(HAENDLER)),
                    (5_596, 4),
                    (10_000, 10_000),
                    17_988,
                )
            },
        }]
    );
}

// --- Der DTO-Vertrag ------------------------------------------------------------

#[test]
fn der_dto_vertrag_nimmt_buchungen_und_gibt_cashflow_ranking_und_kandidaten_ohne_datei_und_netz() {
    // Buchungen rein — darunter eine Wiederkehr, eine Gebühr und eine Umbuchung —, die ganze Antwort
    // raus, in **einem** Vergleich. Alles lebt im Speicher: kein Pfad, keine Datei, kein Netz.
    let api = ein_fenster(&[
        wiederkehrend(JAN_15, 1_299),
        wiederkehrend(FEB_15, 1_299),
        entgelt(FEB_15, 500),
        wiederkehrend(MAR_15, 1_299),
        dauerauftrag(MAR_15, 10_000, SPARKONTO),
    ])
    .with_own_accounts(eigene_konten(&[
        (EIGENES_KONTO, "giro"),
        (SPARKONTO, "spar"),
    ]))
    .with_savings_threshold(schwelle(1_000));

    let schluessel = fingerabdruck(HAENDLER);
    let abo = DriverRankRow {
        rhythm: Some(RecurrenceRhythm::Monthly),
        ..platz(art(&schluessel), (3_897, 3), (8_862, 8_862), 15_588)
    };
    assert_eq!(
        api.run_history(LAUF.0, LAUF.1),
        Ok(HistoryEvaluationRow {
            currency: Some("EUR".to_string()),
            // Die Zeitachse ist ungefiltert — auch die Umbuchung steht darin.
            occurrences: vec![
                vorkommen(&schluessel, JAN_15, 1_299, BookingDirection::Outgoing),
                vorkommen(&schluessel, FEB_15, 1_299, BookingDirection::Outgoing),
                vorkommen(&schluessel, MAR_15, 1_299, BookingDirection::Outgoing),
                vorkommen(
                    "PERSON:PERSON_7",
                    MAR_15,
                    10_000,
                    BookingDirection::Outgoing
                ),
            ],
            recurrences: vec![wiederkehr(
                &schluessel,
                RecurrenceRhythm::Monthly,
                1_299,
                15_588
            )],
            // Der Aderlass dagegen ist genettet: der Dauerauftrag aufs Sparkonto fehlt im März.
            account_cashflows: vec![
                konto_monat(EIGENES_KONTO, Some("giro"), (2024, 1), 0, 1_299),
                konto_monat(EIGENES_KONTO, Some("giro"), (2024, 2), 0, 1_799),
                konto_monat(EIGENES_KONTO, Some("giro"), (2024, 3), 0, 1_299),
            ],
            ranking: vec![
                abo.clone(),
                platz(gebuehren_von(EIGENES_KONTO), (500, 1), (1_138, 10_000), 500),
            ],
            savings_candidates: vec![SavingsCandidateRow {
                pareto_rank: 1,
                ranked: abo,
            }],
        })
    );
}
