//! Black-Box-Spezifikation der **Wiederkehr-Erkennung**
//! ([Issue 0005](../../../docs/issues/0005-wiederkehr-erkennung.md)) — ausschließlich über die public
//! API des Adapters (Fluent-`HistoryTestApi` + DTOs + der re-exportierte Domänen-`EvaluationError`).
//!
//! Sie sitzt auf der **fenster-übergreifenden** Reichweite (ADR-0013, Entscheidung E): erst die Sicht
//! über **alle** Fenster des Laufs belegt einen Rhythmus — ein jährliches Abo erscheint je ~400-Tage-
//! Seite nur einmal. Das Ergebnis wird **zurückgegeben**, nie abgelegt.
//!
//! Was als wiederkehrender Abfluss gilt, ist hier festgehalten, nicht bloß behauptet: gleicher
//! Fingerabdruck, fester Takt (monatlich: Abstand 25–35 Tage, mind. 3 Abbuchungen; jährlich: 350–380
//! Tage, mind. 2), ähnlicher Betrag (±5 % je Preisstufe), nur **Abflüsse**, und **genettet** — eine
//! Umbuchung aufs eigene Konto ist kein Kosten-Treiber.

use auswerten_adapter::{
    BookingDirection, EvaluationError, HistoryEvaluationRow, RecurrenceRhythm, RecurrenceRow,
    WindowBookingRow,
};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{
    APR_15, FEB_15, FEB_29, HAENDLER, JAN_15, JUN_15, MAERZ_2023, MAERZ_2024, MAERZ_2025, MAI_15,
    MAR_15, SPARKONTO, ZWEITES_KONTO, dauerauftrag, eigene_konten, eigene_konten_mit_marker,
    fenster, fenster_mit_marker, fingerabdruck, historie_api, preissteigerung, ueber_paypal,
    vorkommen, wiederkehr, wiederkehrend,
};

/// Die Lauf-Spanne, die alle Tage dieses Specs umfasst — von 2023 bis 2025.
const LAUF: (i64, i64) = (MAERZ_2023, MAERZ_2025);

/// Fährt die Reichweite über die ganze Spanne und gibt **nur** die Wiederkehr-Liste zurück.
fn wiederkehr_in(
    api: &auswerten_adapter::HistoryTestApi,
) -> Result<Vec<RecurrenceRow>, EvaluationError> {
    api.run_history(LAUF.0, LAUF.1)
        .map(|auswertung| auswertung.recurrences)
}

// --- Monatlich ------------------------------------------------------------------

#[test]
fn eine_monatliche_serie_gleicher_ausgaenge_ist_ein_wiederkehrender_abfluss() {
    // Fünf Abbuchungen desselben Händlers zum selben Betrag, Monat für Monat — über **zwei** Fenster
    // verteilt, wie der Pager sie geschnitten haben könnte. Der Takt hält über die Fenstergrenze.
    let api = historie_api()
        .with_window(
            "fenster-1",
            fenster(&[
                wiederkehrend(JAN_15, 1_299),
                wiederkehrend(FEB_15, 1_299),
                wiederkehrend(MAR_15, 1_299),
            ]),
        )
        .with_window(
            "fenster-2",
            fenster(&[wiederkehrend(APR_15, 1_299), wiederkehrend(MAI_15, 1_299)]),
        );

    // Der **Jahresbetrag** ist der typische Betrag mal zwölf — die Bezugsgröße für spätere
    // Spar-Kandidaten.
    assert_eq!(
        wiederkehr_in(&api),
        Ok(vec![wiederkehr(
            &fingerabdruck(HAENDLER),
            RecurrenceRhythm::Monthly,
            1_299,
            15_588
        )])
    );
}

#[test]
fn ein_leicht_schwankender_betrag_ist_dieselbe_wiederkehr_ohne_preissteigerung() {
    // „Ähnlicher Betrag", nicht „gleicher": Schwankungen innerhalb von ±5 % (Rundung, Kursreste)
    // bleiben **eine** Preisstufe. Der typische Betrag ist ihr Median — ein wirklich abgebuchter
    // Betrag, kein gerundeter Mittelwert.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            wiederkehrend(JAN_15, 1_000),
            wiederkehrend(FEB_15, 1_020),
            wiederkehrend(MAR_15, 990),
            wiederkehrend(APR_15, 1_010),
        ]),
    );

    assert_eq!(
        wiederkehr_in(&api),
        Ok(vec![wiederkehr(
            &fingerabdruck(HAENDLER),
            RecurrenceRhythm::Monthly,
            1_000,
            12_000
        )])
    );
}

#[test]
fn zwei_monatliche_abbuchungen_sind_noch_keine_wiederkehr() {
    // Zwei Käufe beim selben Händler im Monatsabstand können Zufall sein; erst die dritte Abbuchung
    // auf demselben Takt ist ein Muster.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[wiederkehrend(JAN_15, 1_299), wiederkehrend(FEB_15, 1_299)]),
    );

    assert_eq!(wiederkehr_in(&api), Ok(vec![]));
}

#[test]
fn ein_unregelmaessiger_abstand_ist_keine_wiederkehr() {
    // Januar, Februar, dann erst wieder April: der Abstand von zwei Monaten fällt aus dem Takt. Die
    // Erkennung behauptet keinen Rhythmus, den die Daten nicht tragen.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            wiederkehrend(JAN_15, 1_299),
            wiederkehrend(FEB_15, 1_299),
            wiederkehrend(APR_15, 1_299),
        ]),
    );

    assert_eq!(wiederkehr_in(&api), Ok(vec![]));
}

// --- Jährlich -------------------------------------------------------------------

#[test]
fn eine_jaehrliche_abbuchung_im_zwoelf_monats_abstand_ist_wiederkehrend() {
    // Ein Jahresabo — je Fenster (~400 Tage) erscheint es nur **einmal**. Erst die Sicht über beide
    // Fenster des Laufs zeigt den Zwölf-Monats-Abstand.
    let api = historie_api()
        .with_window("fenster-2023", fenster(&[wiederkehrend(MAERZ_2023, 8_999)]))
        .with_window("fenster-2024", fenster(&[wiederkehrend(MAERZ_2024, 8_999)]));

    // Der Jahresbetrag eines Jahresabos ist sein Betrag — einmal im Jahr.
    assert_eq!(
        wiederkehr_in(&api),
        Ok(vec![wiederkehr(
            &fingerabdruck(HAENDLER),
            RecurrenceRhythm::Yearly,
            8_999,
            8_999
        )])
    );
}

#[test]
fn ein_jahresabo_mit_verschobenem_abbuchungstag_bleibt_jaehrlich_und_zeigt_seine_preissteigerung() {
    // Drei Jahre, am Ende teurer — und der dritte Abbuchungstag rutschte übers Wochenende. Beides
    // bricht den jährlichen Takt nicht.
    let api = historie_api()
        .with_window("fenster-2023", fenster(&[wiederkehrend(MAERZ_2023, 8_999)]))
        .with_window("fenster-2024", fenster(&[wiederkehrend(MAERZ_2024, 8_999)]))
        .with_window(
            "fenster-2025",
            fenster(&[wiederkehrend(MAERZ_2025, 10_999)]),
        );

    assert_eq!(
        wiederkehr_in(&api),
        Ok(vec![RecurrenceRow {
            price_increase: Some(preissteigerung(MAERZ_2025, 8_999, 10_999)),
            ..wiederkehr(
                &fingerabdruck(HAENDLER),
                RecurrenceRhythm::Yearly,
                10_999,
                10_999
            )
        }])
    );
}

// --- Preissteigerung ------------------------------------------------------------

#[test]
fn eine_preissteigerung_ab_einem_stichtag_wird_erkannt_und_ausgewiesen() {
    // Drei Monate zu 12,99 €, ab April 14,99 € — derselbe Händler, derselbe Takt. Ausgewiesen wird
    // der **Stichtag** samt altem und neuem Betrag; typischer Betrag und Jahresbetrag rechnen mit
    // dem **neuen** Preis, denn der geht künftig ab.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            wiederkehrend(JAN_15, 1_299),
            wiederkehrend(FEB_15, 1_299),
            wiederkehrend(MAR_15, 1_299),
            wiederkehrend(APR_15, 1_499),
            wiederkehrend(MAI_15, 1_499),
        ]),
    );

    assert_eq!(
        wiederkehr_in(&api),
        Ok(vec![RecurrenceRow {
            price_increase: Some(preissteigerung(APR_15, 1_299, 1_499)),
            ..wiederkehr(
                &fingerabdruck(HAENDLER),
                RecurrenceRhythm::Monthly,
                1_499,
                17_988
            )
        }])
    );
}

#[test]
fn ein_gesunkener_betrag_bricht_die_serie() {
    // Eine Senkung ist keine Preissteigerung — und ein Betrag, der mal steigt, mal fällt, ist kein
    // gleichbleibender Abfluss. Die Erkennung weist ihn lieber **nicht** aus, als ein Abo zu
    // behaupten, das es so nicht gibt.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            wiederkehrend(JAN_15, 1_499),
            wiederkehrend(FEB_15, 1_499),
            wiederkehrend(MAR_15, 1_499),
            wiederkehrend(APR_15, 999),
        ]),
    );

    assert_eq!(wiederkehr_in(&api), Ok(vec![]));
}

#[test]
fn eine_stufe_mit_gesunkenem_typischem_betrag_ist_keine_preissteigerung() {
    // 100,00 / 105,00 / 105,00 / 106,00 / 101,00 / 101,00: die 106,00 liegen über der Toleranz der
    // ersten Stufe und beginnen eine neue — aber deren **typischer** Betrag (101,00) liegt **unter**
    // dem der ersten (105,00). Das ist eine Senkung, keine Steigerung, und die Reihe als Ganzes
    // schwankt. Sie wird darum gar nicht ausgewiesen — statt als „Preissteigerung von 105,00 auf
    // 101,00".
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            wiederkehrend(JAN_15, 10_000),
            wiederkehrend(FEB_15, 10_500),
            wiederkehrend(MAR_15, 10_500),
            wiederkehrend(APR_15, 10_600),
            wiederkehrend(MAI_15, 10_100),
            wiederkehrend(JUN_15, 10_100),
        ]),
    );

    assert_eq!(wiederkehr_in(&api), Ok(vec![]));
}

// --- Bewusste Grenze: eine Art, eine Reihe ---------------------------------------

#[test]
fn ein_einzelkauf_beim_selben_glaeubiger_bricht_die_reihe() {
    // Ein Monatsabo und dazwischen ein einzelner Kauf beim **selben** Gläubiger: beide tragen
    // denselben Fingerabdruck, landen also in **einer** Reihe — und der Einzelkauf zerreißt deren
    // Takt (Abstand 14 Tage). Die Erkennung trennt das nicht auf (kein Sub-Clustering); sie weist
    // lieber nichts aus als einen Rhythmus, den die Reihe als Ganzes nicht trägt. Bewusste Grenze,
    // im Ticket festgehalten.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            wiederkehrend(JAN_15, 1_299),
            wiederkehrend(FEB_15, 1_299),
            wiederkehrend(FEB_29, 4_999),
            wiederkehrend(MAR_15, 1_299),
            wiederkehrend(APR_15, 1_299),
        ]),
    );

    assert_eq!(wiederkehr_in(&api), Ok(vec![]));
}

// --- Zahlungsintermediär --------------------------------------------------------

#[test]
fn zwei_haendler_hinter_demselben_durchleiter_ergeben_zwei_wiederkehr_gruppen() {
    // Zwei Abos, beide über PayPal abgerechnet — die Gegenpartei ist jedes Mal **dieselbe**
    // Gläubiger-ID. Hinge die Wiederkehr am Durchleiter, verklumpten sie zu einer Gruppe mit zwei
    // Abbuchungen je Monat (Abstand 0 → gar kein Takt). Sie hängt am **Händler-Token** des
    // Verwendungszwecks; die je Buchung wechselnde Referenznummer spaltet ihn nicht.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            ueber_paypal(JAN_15, 1_099, "PP.1001.PP Spotify AB"),
            ueber_paypal(JAN_15, 1_799, "PP.2001.PP Streamflix"),
            ueber_paypal(FEB_15, 1_099, "PP.1002.PP Spotify AB"),
            ueber_paypal(FEB_15, 1_799, "PP.2002.PP Streamflix"),
            ueber_paypal(MAR_15, 1_099, "PP.1003.PP Spotify AB"),
            ueber_paypal(MAR_15, 1_799, "PP.2003.PP Streamflix"),
        ]),
    );

    assert_eq!(
        wiederkehr_in(&api),
        Ok(vec![
            wiederkehr(
                "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:PP-PP-SPOTIFY-AB",
                RecurrenceRhythm::Monthly,
                1_099,
                13_188
            ),
            wiederkehr(
                "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:PP-PP-STREAMFLIX",
                RecurrenceRhythm::Monthly,
                1_799,
                21_588
            ),
        ])
    );
}

// --- Genettet: nur echte Kosten-Treiber -----------------------------------------

#[test]
fn eine_wiederkehrende_umbuchung_auf_ein_eigenes_konto_erscheint_nicht() {
    // Ein Dauerauftrag aufs **eigene** Sparkonto: Monat für Monat derselbe Betrag, perfekter Takt —
    // und trotzdem kein Kosten-Treiber, sondern eine Umbuchung (Issue 0004). Dieselbe Token-Regel,
    // die sie aus dem Cashflow nimmt, nimmt sie aus der Wiederkehr.
    let api = historie_api()
        .with_own_accounts(eigene_konten(&[(SPARKONTO, "spar")]))
        .with_window(
            "fenster-1",
            fenster(&[
                dauerauftrag(JAN_15, 20_000, SPARKONTO),
                dauerauftrag(FEB_15, 20_000, SPARKONTO),
                dauerauftrag(MAR_15, 20_000, SPARKONTO),
            ]),
        );

    let auswertung = api.run_history(LAUF.0, LAUF.1);

    // Heraus aus der Wiederkehr — aber **nicht** aus der Zeitachse: die Vorkommen bleiben die
    // ungefilterte Sicht, die Umbuchung kam vor.
    assert_eq!(
        auswertung.as_ref().map(|a| a.recurrences.clone()),
        Ok(vec![])
    );
    assert_eq!(auswertung.map(|a| a.occurrences.len()), Ok(3));
}

#[test]
fn derselbe_dauerauftrag_an_ein_fremdes_konto_ist_ein_wiederkehrender_abfluss() {
    // Die Gegenprobe zur Umbuchung: dieselben Buchungen, aber das Gegenkonto ist **nicht** meines
    // (die Miete an die Vermieterin). Erst damit ist belegt, dass es das Netting ist, das die
    // Umbuchung oben verschwinden lässt — und nicht etwa die Privatperson als Gegenpartei.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            dauerauftrag(JAN_15, 95_000, "KONTO_FREMD"),
            dauerauftrag(FEB_15, 95_000, "KONTO_FREMD"),
            dauerauftrag(MAR_15, 95_000, "KONTO_FREMD"),
        ]),
    );

    assert_eq!(
        wiederkehr_in(&api),
        Ok(vec![wiederkehr(
            "PERSON:PERSON_7",
            RecurrenceRhythm::Monthly,
            95_000,
            1_140_000
        )])
    );
}

#[test]
fn ein_regelmaessiger_zufluss_ist_kein_kosten_treiber() {
    // Das Gehalt kommt pünktlich jeden Monat — aber es kostet nichts. Nur **Abflüsse** sind
    // Kosten-Treiber.
    let zufluss = |day| WindowBookingRow {
        direction: BookingDirection::Incoming,
        ..wiederkehrend(day, 320_000)
    };
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[zufluss(JAN_15), zufluss(FEB_15), zufluss(MAR_15)]),
    );

    assert_eq!(wiederkehr_in(&api), Ok(vec![]));
}

#[test]
fn eine_eigene_konten_config_aus_anderer_tokenisierung_endet_terminal() {
    // Stammen Historie und Config aus verschiedenen Tokenisierungen, matchte kein Token — jede
    // Umbuchung liefe lautlos als „fremd" durch und erschiene als Abo. Lieber gar kein Ergebnis als
    // eines, das sich richtig liest und falsch ist.
    let api = historie_api()
        .with_own_accounts(eigene_konten_mit_marker("marker-b", &[(SPARKONTO, "spar")]))
        .with_window(
            "fenster-1",
            fenster(&[
                dauerauftrag(JAN_15, 20_000, SPARKONTO),
                dauerauftrag(FEB_15, 20_000, SPARKONTO),
                dauerauftrag(MAR_15, 20_000, SPARKONTO),
            ]),
        );

    assert_eq!(wiederkehr_in(&api), Err(EvaluationError::SeedMismatch));
}

#[test]
fn eine_historie_ohne_seed_marker_endet_terminal_und_redigiert() {
    // Liegen Buchungen in der Spanne, ist der Seed-Marker **Pflicht**: ohne ihn ließe sich nicht
    // prüfen, ob die Gegenkonto-Token zur Eigene-Konten-Config passen, und das Netting liefe blind.
    // Das endet terminal — mit dem statischen Feldnamen, nie mit einem Rohwert.
    let api = historie_api().with_window(
        "fenster-1",
        fenster_mit_marker(
            "",
            &[
                wiederkehrend(JAN_15, 1_299),
                wiederkehrend(FEB_15, 1_299),
                wiederkehrend(MAR_15, 1_299),
            ],
        ),
    );

    assert_eq!(
        wiederkehr_in(&api),
        Err(EvaluationError::MissingMandatoryField {
            field: "seed_marker"
        })
    );
}

#[test]
fn eine_leere_lauf_spanne_braucht_keinen_seed_abgleich() {
    // Liegt in der Spanne nichts, gibt es kein Token, das sich falsch einordnen ließe — ein Fenster
    // **außerhalb** des Laufs mit fremdem Marker darf den Lauf nicht scheitern lassen.
    let api = historie_api().with_window(
        "lauf-davor",
        fenster_mit_marker("marker-b", &[wiederkehrend(JAN_15, 1_299)]),
    );

    assert_eq!(
        api.run_history(MAERZ_2024, MAERZ_2025),
        Ok(HistoryEvaluationRow::default())
    );
}

// --- Der DTO-Vertrag ------------------------------------------------------------

#[test]
fn der_dto_vertrag_nimmt_buchungen_und_gibt_das_wiederkehr_ergebnis_ohne_datei_und_netz() {
    // Buchungs-Liste rein, Wiederkehr-Ergebnis raus — die ganze Antwort in **einem** Vergleich. Die
    // TestApi läuft vollständig im Speicher: kein Pfad, keine Datei, kein Netz; was der Slice
    // braucht, kommt als DTO herein, und was er weiß, geht als DTO hinaus.
    //
    // Das Abo zog im März aufs zweite Konto um — belastet ist das Konto der **jüngsten** Abbuchung,
    // denn dort müsste eine Kündigung ansetzen.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            wiederkehrend(JAN_15, 1_299),
            wiederkehrend(FEB_15, 1_299),
            WindowBookingRow {
                ordering_account: ZWEITES_KONTO.to_string(),
                ..wiederkehrend(MAR_15, 1_299)
            },
        ]),
    );

    let schluessel = fingerabdruck(HAENDLER);
    assert_eq!(
        api.run_history(LAUF.0, LAUF.1),
        Ok(HistoryEvaluationRow {
            occurrences: vec![
                vorkommen(&schluessel, JAN_15, 1_299, BookingDirection::Outgoing),
                vorkommen(&schluessel, FEB_15, 1_299, BookingDirection::Outgoing),
                vorkommen(&schluessel, MAR_15, 1_299, BookingDirection::Outgoing),
            ],
            recurrences: vec![RecurrenceRow {
                charged_account: ZWEITES_KONTO.to_string(),
                ..wiederkehr(&schluessel, RecurrenceRhythm::Monthly, 1_299, 15_588)
            }],
        })
    );
}
