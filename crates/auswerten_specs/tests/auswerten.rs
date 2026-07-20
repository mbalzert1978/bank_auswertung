//! Black-Box-Spezifikation des auswerten-Slice — ausschließlich über die public API des Adapters
//! (Fluent-`EvaluationTestApi` + DTOs + der re-exportierte Domänen-`EvaluationError`), nie über
//! Interna. `run` fährt die **echte** Kette (`EvaluateWindowCommandDecoder` →
//! `WindowEvaluationHandler`) gegen öffentliche Fakes und gibt den **nackten Domänen-Fehler**
//! zurück — keine Outbox-Typen (die Outbox-Mechanik prüft der io-Integrationstest).
//!
//! Der Slice ist der vierte Pipeline-Schritt und trägt **zwei Reichweiten** (ADR-0013,
//! Entscheidung E). Dieser Spec deckt die **erste** ab: den Pro-Fenster-Handler auf
//! `FensterGeschlossen`. Er bekommt **nur die ID** des eingefrorenen Auswertungsfensters, liest das
//! Aggregat darüber per Port und rechnet die erste periodische Kennzahl **innerhalb** des Fensters:
//! den **Cashflow je Monat**. Das Fenster ist dabei die **Seitengröße** des Pagers, **nicht** der
//! Bucketing-Zeitraum (CONTEXT.md, „Fenster vs. Zeitraum") — ein Fenster deckt viele Monate ab.

use auswerten_adapter::{AnalysisWindowRow, EvaluationError, WindowEvaluationRow};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{
    FEB_01, FEB_29, FENSTER, JAN_15, JAN_31, MAR_05, api, ausgang, eingang, fenster, firma, monat,
    person,
};

// --- Kern-Pfad: ID herein → Aggregat gelesen → Pro-Fenster-Ergebnis abgelegt ----

#[test]
fn ein_geschlossenes_fenster_wird_per_id_gelesen_und_als_pro_fenster_ergebnis_abgelegt() {
    // Der Handler bekommt einen **Zeiger**, keine eingebetteten Daten (uniformes ID-Handoff,
    // ADR-0012) — das Aggregat holt er selbst über den Lese-Port.
    let api = api().with_window(
        FENSTER,
        fenster(&[ausgang(JAN_15, 4_200), eingang(JAN_31, 250_000)]),
    );

    let ergebnis = api.run(FENSTER);

    assert_eq!(ergebnis, Ok(()));
    // Genau **ein** abgelegtes Ergebnis, und es trägt die Quell-ID als Fremdschlüssel: Zu- und
    // Abfluss stehen getrennt und je vorzeichenlos (ADR-0005), gebucketet auf den Monat der Tage.
    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 250_000, 4_200)],
        // Buchungen ohne Gegenpartei (Entgeltzeilen) tragen keinen Fingerabdruck bei.
        fingerprints: vec![],
    });
}

#[test]
fn der_handler_wertet_genau_das_fenster_aus_auf_das_der_zeiger_zeigt() {
    // Zwei eingefrorene Fenster liegen in der Ablage — herein kommt **nur eine ID**. Dass genau
    // deren Daten im Ergebnis stehen (und die des anderen Fensters nirgends), ist der Beleg, dass
    // der Handler **per ID liest** statt ein eingebettetes Aggregat zu bekommen.
    let api = api()
        .with_window(FENSTER, fenster(&[ausgang(JAN_15, 1_100)]))
        .with_window("fenster-2", fenster(&[ausgang(JAN_15, 9_900)]));

    api.run(FENSTER).expect("das adressierte Fenster");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 1_100)],
        fingerprints: vec![],
    });
}

// --- Der Zeitraum ist der Monat, nicht das Fenster -----------------------------

#[test]
fn der_cashflow_wird_je_monat_innerhalb_des_fensters_gebucketet() {
    // **Ein** Fenster, **mehrere** Monats-Buckets: das Fenster ist die Seitengröße des Pagers
    // (~400 Tage), nicht der Berichtszeitraum (CONTEXT.md, „Fenster vs. Zeitraum"). Die vier
    // Buchungen prüfen dabei genau die Kanten der Kalender-Rekonstruktion: die Monatsgrenze
    // (31.01. → 01.02.), den Schalttag (29.02.2024) und das Zusammenfalten zweier Buchungen
    // desselben Monats.
    let api = api().with_window(
        FENSTER,
        fenster(&[
            ausgang(JAN_31, 1_000),
            eingang(FEB_01, 5_000),
            ausgang(FEB_29, 2_000),
            ausgang(MAR_05, 300),
        ]),
    );

    api.run(FENSTER).expect("ein Fenster über drei Monate");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        // Chronologisch, und je Bucket stehen Zu- und Abfluss **getrennt und vorzeichenlos**
        // nebeneinander (ADR-0005) — kein zusammengerechneter Saldo.
        months: vec![
            monat(2024, 1, 0, 1_000),
            monat(2024, 2, 5_000, 2_000),
            monat(2024, 3, 0, 300),
        ],
        fingerprints: vec![],
    });
}

#[test]
fn ein_monat_ohne_buchung_ergibt_keinen_bucket() {
    // Januar und März, kein Februar. Ein leerer Bucket wäre eine Behauptung über einen Zeitraum,
    // aus dem nichts vorliegt — „kein Umsatz" und „keine Daten" sind nicht dasselbe.
    let api = api().with_window(
        FENSTER,
        fenster(&[ausgang(JAN_15, 700), ausgang(MAR_05, 900)]),
    );

    api.run(FENSTER).expect("ein Fenster mit Monats-Lücke");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 700), monat(2024, 3, 0, 900)],
        fingerprints: vec![],
    });
}

#[test]
fn ein_absurder_buchungstag_beendet_den_lauf_ohne_panic() {
    // Der Tag-Stamp kommt **roh aus io** — die Domäne hat keine Zusage, dass er plausibel ist. Ein
    // Extremwert an beiden Enden des Wertebereichs darf die Kalender-Rekonstruktion nicht zum
    // Überlauf bringen: im Debug-Build wäre das ein Panic, und die Pipeline kennt nur terminale
    // Fehler, keine Abstürze. Ein absurder Tag ergibt einen absurden, aber **beschränkten** Monat.
    let api = api().with_window(
        FENSTER,
        fenster(&[ausgang(i64::MAX, 100), eingang(i64::MIN, 200)]),
    );

    let ergebnis = api.run(FENSTER);

    // Kein Panic, kein Fehler — der Lauf trägt die Extremwerte durch und legt ab. Welche Monate
    // dabei herauskommen, ist bewusst nicht Gegenstand: für Müll-Eingaben gibt es keine richtige
    // Antwort, nur eine ungefährliche.
    assert_eq!(ergebnis, Ok(()));
}

// --- Der Fingerabdruck: der stabile Schlüssel einer *Art* von Buchung ----------

#[test]
fn zwei_buchungen_desselben_haendlers_ergeben_denselben_fingerabdruck() {
    // Derselbe Händler an zwei Tagen, mit zwei **verschiedenen** Verwendungszwecken. Der
    // Fingerabdruck identifiziert die *Art* der Buchung, nicht die einzelne Buchung — sonst hinge
    // die Wiederkehr-Erkennung an einem Schlüssel, der sich je Zeile ändert.
    let api = api().with_window(
        FENSTER,
        fenster(&[
            firma(
                JAN_15,
                1_999,
                "DE98ZZZ09999999999",
                "Netflix",
                Some("Abo Januar"),
            ),
            firma(
                FEB_29,
                1_999,
                "DE98ZZZ09999999999",
                "Netflix",
                Some("Abo Februar"),
            ),
        ]),
    );

    api.run(FENSTER)
        .expect("zwei Buchungen desselben Haendlers");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 1_999), monat(2024, 2, 0, 1_999)],
        // **Ein** Eintrag, nicht zwei: die zwei Buchungen sind eine Art.
        fingerprints: vec!["COMPANY:DE98ZZZ09999999999".to_string()],
    });
}

#[test]
fn verschiedene_haendler_hinter_demselben_intermediaer_ergeben_verschiedene_fingerabdruecke() {
    // Beide Buchungen tragen **dieselbe** Gegenpartei — den Zahlungsintermediär PayPal, mit seiner
    // einen Gläubiger-ID. Schlüsselte der Fingerabdruck nur auf sie, fasste der Durchleiter alle
    // dahinterliegenden Händler zu einem Klumpen zusammen und die Wiederkehr hinge am falschen
    // Schlüssel. Der Händler steckt hier allein im Verwendungszweck.
    let api = api().with_window(
        FENSTER,
        fenster(&[
            firma(
                JAN_15,
                4_235,
                "LU96ZZZ0000000000000000058",
                "PayPal Europe S.a.r.l. et Cie S.C.A.",
                Some("PP.4711.PP . REWE Markt, Ihr Einkauf"),
            ),
            firma(
                JAN_31,
                999,
                "LU96ZZZ0000000000000000058",
                "PayPal Europe S.a.r.l. et Cie S.C.A.",
                Some("PP.4712.PP . Spotify AB, Ihr Einkauf"),
            ),
        ]),
    );

    api.run(FENSTER).expect("zwei Haendler hinter PayPal");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 5_234)],
        // **Zwei** Arten, nicht eine: der Händler-Token des Verwendungszwecks trennt sie.
        fingerprints: vec![
            "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:PP-PP-REWE-MARKT-IHR-EINKAUF".to_string(),
            "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:PP-PP-SPOTIFY-AB-IHR-EINKAUF".to_string(),
        ],
    });
}

#[test]
fn derselbe_haendler_hinter_einem_intermediaer_ueberlebt_die_wechselnde_referenznummer() {
    // Die Kehrseite: der Verwendungszweck eines Durchleiters trägt eine **je Buchung wechselnde**
    // Referenznummer. Ginge sie in den Token ein, bekäme jede einzelne Buchung ihren eigenen
    // Schlüssel — der Fingerabdruck identifizierte dann die Buchung statt ihrer Art, und die
    // Wiederkehr-Erkennung fände nie zwei Vorkommen desselben Händlers.
    let api = api().with_window(
        FENSTER,
        fenster(&[
            firma(
                JAN_15,
                1_200,
                "LU96ZZZ0000000000000000058",
                "PayPal",
                Some("PP.4711.PP . REWE Markt"),
            ),
            firma(
                FEB_01,
                1_200,
                "LU96ZZZ0000000000000000058",
                "PayPal",
                Some("PP.9902.PP . REWE Markt"),
            ),
        ]),
    );

    api.run(FENSTER)
        .expect("derselbe Haendler, zwei Referenzen");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 1_200), monat(2024, 2, 0, 1_200)],
        fingerprints: vec![
            "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:PP-PP-REWE-MARKT".to_string(),
        ],
    });
}

#[test]
fn dieselbe_privatperson_ergibt_bei_gleichem_seed_denselben_fingerabdruck() {
    // **Zwei getrennte Läufe = zwei Importe.** Derselbe Seed hat stromaufwärts in `obfuskieren`
    // dasselbe `PERSON_…` vergeben (ADR-0010) — und weil der Fingerabdruck auf dem Pseudonym
    // schlüsselt, bleibt die Dauerauftrags-Art über Importe hinweg dieselbe. Dieser Slice sieht den
    // Seed dabei **nie**; er sieht nur dessen Wirkung.
    let erster_import = api().with_window(FENSTER, fenster(&[person(JAN_15, 25_000, "PERSON_7")]));
    let zweiter_import = api().with_window(FENSTER, fenster(&[person(MAR_05, 25_000, "PERSON_7")]));

    erster_import.run(FENSTER).expect("erster Import");
    zweiter_import.run(FENSTER).expect("zweiter Import");

    erster_import.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 25_000)],
        fingerprints: vec!["PERSON:PERSON_7".to_string()],
    });
    zweiter_import.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 3, 0, 25_000)],
        // **Derselbe** Schlüssel wie im ersten Import — trotz anderen Monats und anderem Lauf.
        fingerprints: vec!["PERSON:PERSON_7".to_string()],
    });
}

#[test]
fn ein_anderer_seed_aendert_den_fingerabdruck_der_privatperson() {
    // Die Kehrseite von ADR-0010: ein instabiler Seed tokenisiert alles neu — `obfuskieren` vergibt
    // derselben Person ein anderes Pseudonym, und damit ändert sich ihr Fingerabdruck. Das ist
    // keine Schwäche des Schlüssels, sondern die Begründung dafür, warum der Seed stabil bleiben
    // muss.
    let api = api().with_window(FENSTER, fenster(&[person(JAN_15, 25_000, "PERSON_42")]));

    api.run(FENSTER).expect("Import mit anderem Seed");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 25_000)],
        // Ein **anderer** Schlüssel als `PERSON:PERSON_7` derselben Person oben.
        fingerprints: vec!["PERSON:PERSON_42".to_string()],
    });
}

// --- Ein Zeiger ins Leere endet terminal und redigiert -------------------------

#[test]
fn ein_fehlendes_aggregat_scheitert_terminal_und_redigiert() {
    // Der Zeiger zeigt ins Leere — das ist terminal, keine Abwesenheit zum Weiterrechnen. Ein
    // Match statt `assert_eq!`, weil die opake ID ein Domänen-Internum ist, das die Black-Box nicht
    // konstruieren kann.
    let api = api();

    let fehler = api.run(FENSTER).expect_err("erwartet terminalen Fehler");

    assert!(
        matches!(fehler, EvaluationError::WindowNotFound(_)),
        "erwartet terminalen WindowNotFound: {fehler:?}"
    );
    // Die Begründung trägt **nur die opake ID** — keinen Rohwert aus dem Datenbestand.
    assert_eq!(
        fehler.to_string(),
        format!("referenced analysis window not found: {FENSTER}")
    );
    api.assert_nothing_stored();
}

#[test]
fn ein_fehlendes_pflichtfeld_scheitert_terminal_und_redigiert() {
    // Das Fenster liegt in der Ablage, aber ein Pflichtfeld (Währung) ist leer — terminal, und die
    // Begründung trägt **nur den statischen Feldnamen**, nie einen Rohwert.
    let api = api().with_window(
        FENSTER,
        AnalysisWindowRow {
            currency: String::new(),
            bookings: vec![ausgang(JAN_15, 4_200)],
        },
    );

    let fehler = api.run(FENSTER).expect_err("erwartet terminalen Fehler");

    assert_eq!(
        fehler,
        EvaluationError::MissingMandatoryField { field: "currency" }
    );
    assert_eq!(
        fehler.to_string(),
        "mandatory field missing in analysis window: currency"
    );
    api.assert_nothing_stored();
}

#[test]
fn eine_firma_ohne_glaeubiger_id_scheitert_terminal_und_redigiert() {
    // Die Schlüssel-Zelle der Gegenpartei ist ein **Pflichtfeld**: ohne sie hätte der Fingerabdruck
    // keinen Schlüssel, über den er stabil bliebe, und stillschweigend auf einen leeren Schlüssel
    // auszuweichen verklumpte alle solchen Buchungen zu einer Art. Die Begründung trägt **nur** den
    // statischen Feldnamen, nie den Rohwert.
    let api = api().with_window(
        FENSTER,
        fenster(&[firma(JAN_15, 1_999, "", "Netflix", Some("Abo"))]),
    );

    let fehler = api.run(FENSTER).expect_err("erwartet terminalen Fehler");

    assert_eq!(
        fehler,
        EvaluationError::MissingMandatoryField {
            field: "creditor_id"
        }
    );
    assert_eq!(
        fehler.to_string(),
        "mandatory field missing in analysis window: creditor_id"
    );
    api.assert_nothing_stored();
}

#[test]
fn eine_privatperson_ohne_pseudonym_scheitert_terminal_und_redigiert() {
    // Dieselbe Regel auf der anderen Seite des Summentyps — und hier zusätzlich datenschutzrelevant:
    // eine Person **ohne** Pseudonym wäre eine Person, die `obfuskieren` nicht tokenisiert hat.
    let api = api().with_window(FENSTER, fenster(&[person(JAN_15, 25_000, "  ")]));

    let fehler = api.run(FENSTER).expect_err("erwartet terminalen Fehler");

    assert_eq!(
        fehler,
        EvaluationError::MissingMandatoryField { field: "pseudonym" }
    );
    api.assert_nothing_stored();
}

// --- Beide I/O-Richtungen sind fehlbar und terminal (ADR-0012) -----------------

#[test]
fn ein_fehlschlag_beim_lesen_beendet_den_lauf_terminal() {
    let api = api()
        .with_window(FENSTER, fenster(&[ausgang(JAN_15, 4_200)]))
        .with_failing_source("Verbindung zur Ablage abgebrochen");

    let ergebnis = api.run(FENSTER);

    assert_eq!(
        ergebnis,
        Err(EvaluationError::SourceUnavailable {
            reason: "Verbindung zur Ablage abgebrochen".to_string()
        })
    );
    api.assert_nothing_stored();
}

#[test]
fn ein_fehlschlag_beim_ablegen_beendet_den_lauf_terminal() {
    let api = api()
        .with_window(FENSTER, fenster(&[ausgang(JAN_15, 4_200)]))
        .with_failing_store("Tabelle gesperrt");

    let ergebnis = api.run(FENSTER);

    // Kein Dead-Letter, kein Retry im Slice — der nackte Domänen-Fehler steigt durch.
    assert_eq!(
        ergebnis,
        Err(EvaluationError::StorageUnavailable {
            reason: "Tabelle gesperrt".to_string()
        })
    );
    api.assert_nothing_stored();
}
