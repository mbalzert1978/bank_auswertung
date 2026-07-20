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
    FEB_01, FEB_29, FENSTER, JAN_15, JAN_31, MAR_05, api, ausgang, eingang, fenster, monat,
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
