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
//! Die **Fingerabdruck**-Specs desselben Handlers (Issues 0041/0047) liegen thematisch getrennt in
//! `fingerabdruck.rs`.

use auswerten_adapter::{
    AnalysisWindowRow, CounterAccountRow, EvaluationError, WindowBookingRow, WindowEvaluationRow,
};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{
    FEB_01, FEB_29, FENSTER, HAENDLER, JAN_15, JAN_31, MAR_05, SEED_MARKER, anfrage, api, ausgang,
    eigen, eigene_konten, eigene_konten_mit_marker, eingang, fenster, fenster_mit_marker,
    fingerabdruck, firma, firma_ohne_namen, gegenkonto, kategorie, monat, person,
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
        counter_accounts: vec![],
        categories: vec![],
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
        counter_accounts: vec![],
        categories: vec![],
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
        counter_accounts: vec![],
        categories: vec![],
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
        counter_accounts: vec![],
        categories: vec![],
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

// --- Gegenkonto: Eigen(Rolle) oder Fremd, aufgelöst auf Token ------------------

#[test]
fn ein_gegenkonto_aus_der_eigen_menge_wird_als_eigen_samt_rolle_aufgeloest() {
    // Die Auflösung arbeitet **auf Token** (ADR-0013, Entscheidung F): das Gegenkonto-Token der
    // Buchung wird gegen die mit **demselben Seed** tokenisierte Eigen-Menge geprüft. Die Rolle
    // bleibt dabei erhalten, weil sie die Deutung trägt — ein Zufluss aufs Spar-Konto ist Sparen,
    // nicht Konsum. Eine rohe IBAN sieht dieser Slice zu keinem Zeitpunkt.
    let api = api()
        .with_own_accounts(eigene_konten(&[("KONTO_1", "spar")]))
        .with_window(FENSTER, fenster(&[gegenkonto(JAN_15, 50_000, "KONTO_1")]));

    api.run(FENSTER).expect("Umbuchung aufs eigene Sparkonto");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 50_000)],
        fingerprints: vec![],
        counter_accounts: vec![eigen("spar")],
        categories: vec![],
    });
}

#[test]
fn jedes_gegenkonto_ausserhalb_der_eigen_menge_wird_als_fremd_aufgeloest() {
    // Die Kehrseite: alles, was nicht in der Eigen-Menge steht, ist fremd — auch ein Token, das
    // einem eigenen **ähnlich** sieht. Verglichen wird das ganze Token, nicht ein Präfix.
    let api = api()
        .with_own_accounts(eigene_konten(&[("KONTO_1", "giro")]))
        .with_window(
            FENSTER,
            fenster(&[
                gegenkonto(JAN_15, 3_000, "KONTO_2"),
                gegenkonto(JAN_31, 4_000, "KONTO_11"),
            ]),
        );

    api.run(FENSTER).expect("zwei fremde Gegenkonten");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 7_000)],
        fingerprints: vec![],
        // **Ein** Eintrag, nicht zwei: die Einordnung ist eine Art, keine Buchungs-Identität.
        counter_accounts: vec![CounterAccountRow::Foreign],
        categories: vec![],
    });
}

#[test]
fn eine_buchung_ohne_gegenkonto_traegt_keine_einordnung_bei() {
    // Eine Entgeltzeile hat **keine** Gegenseite. Sie als „fremd" zu führen wäre eine Behauptung
    // über ein Konto, das es nicht gibt — „keine Gegenseite" und „eine fremde Gegenseite" sind
    // nicht dasselbe (dieselbe Linie, die der Fingerabdruck bei fehlender Gegenpartei zieht).
    let api = api()
        .with_own_accounts(eigene_konten(&[("KONTO_1", "giro")]))
        .with_window(FENSTER, fenster(&[ausgang(JAN_15, 450)]));

    api.run(FENSTER).expect("eine Entgeltzeile");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 450)],
        fingerprints: vec![],
        counter_accounts: vec![],
        categories: vec![],
    });
}

#[test]
fn ohne_eigene_konten_ist_jedes_gegenkonto_fremd() {
    // Eine **leere** Eigen-Menge ist zulässig und kein Fehler: wer keine eigenen Konten führt, hat
    // schlicht nur fremde Gegenkonten. Das ist eine Aussage, keine Lücke.
    let api = api().with_window(FENSTER, fenster(&[gegenkonto(JAN_15, 3_000, "KONTO_1")]));

    api.run(FENSTER).expect("leere Eigen-Menge");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 3_000)],
        fingerprints: vec![],
        counter_accounts: vec![CounterAccountRow::Foreign],
        categories: vec![],
    });
}

// --- Eine fehlerhafte Eigene-Konten-Config wird abgelehnt ----------------------

#[test]
fn eine_config_mit_unbekannter_rolle_wird_terminal_abgelehnt() {
    // Kein stillschweigendes Überspringen: eine Zeile mit unbekannter Rolle zu ignorieren führte
    // das eigene Konto klammheimlich als *fremd* — die Umbuchung darauf sähe wie Konsum aus, und
    // niemand erführe davon. Die Rolle trägt die Deutung, also ist ihr Fehlen terminal.
    let api = api()
        .with_own_accounts(eigene_konten(&[
            ("KONTO_1", "giro"),
            ("KONTO_2", "tagesgeld"),
        ]))
        .with_window(FENSTER, fenster(&[gegenkonto(JAN_15, 3_000, "KONTO_1")]));

    let fehler = api.run(FENSTER).expect_err("erwartet terminalen Fehler");

    assert_eq!(fehler, EvaluationError::UnknownAccountRole);
    // Die Begründung trägt **keinen** Rohwert — auch nicht die verrutschte Rollen-Zelle selbst:
    // sie kann beliebigen Inhalt führen (im schlimmsten Fall eine dorthin verirrte IBAN).
    assert_eq!(
        fehler.to_string(),
        "unknown account role in the own-accounts configuration"
    );
    api.assert_nothing_stored();
}

#[test]
fn eine_config_mit_leerem_token_wird_terminal_abgelehnt() {
    // Dieselbe Regel auf der anderen Zelle: ein eigenes Konto **ohne** Token wäre ein Eintrag, gegen
    // den nie etwas matchen kann — eine Eigen-Menge, die stillschweigend eines ihrer Konten
    // verliert. Redigiert: **nur** der statische Feldname.
    let api = api()
        .with_own_accounts(eigene_konten(&[("  ", "giro")]))
        .with_window(FENSTER, fenster(&[gegenkonto(JAN_15, 3_000, "KONTO_1")]));

    let fehler = api.run(FENSTER).expect_err("erwartet terminalen Fehler");

    assert_eq!(
        fehler,
        EvaluationError::MissingMandatoryField { field: "token" }
    );
    api.assert_nothing_stored();
}

#[test]
fn eine_config_ohne_seed_marker_wird_terminal_abgelehnt() {
    // Der Marker ist **Pflicht**, nicht Kür: eine Eigen-Menge ohne Kennzeichen ihrer Tokenisierung
    // ließe sich gegen kein Fenster prüfen — der Vergleich, der die ganze Auflösung trägt, wäre
    // unbelegt. Sie durchzulassen hieße, genau die Absicherung zu umgehen, für die der Marker da
    // ist. Redigiert: **nur** der statische Feldname.
    let api = api()
        .with_own_accounts(eigene_konten_mit_marker("  ", &[("KONTO_1", "giro")]))
        .with_window(FENSTER, fenster(&[gegenkonto(JAN_15, 3_000, "KONTO_1")]));

    let fehler = api.run(FENSTER).expect_err("erwartet terminalen Fehler");

    assert_eq!(
        fehler,
        EvaluationError::MissingMandatoryField {
            field: "seed_marker"
        }
    );
    api.assert_nothing_stored();
}

// --- Derselbe Seed trägt die Zuordnung; ein anderer bricht sie sichtbar --------

#[test]
fn dieselbe_tokenisierung_ordnet_ueber_importe_hinweg_stabil_zu() {
    // **Zwei getrennte Läufe = zwei Importe.** Derselbe Seed hat stromaufwärts dasselbe `KONTO_…`
    // vergeben (ADR-0010) — und weil die Auflösung auf genau diesem Token vergleicht, bleibt das
    // eigene Sparkonto über Importe hinweg dasselbe eigene Sparkonto. Der Seed selbst erreicht
    // diesen Slice dabei nie; er sieht nur dessen Wirkung.
    let erster_import = api()
        .with_own_accounts(eigene_konten(&[("KONTO_9", "spar")]))
        .with_window(FENSTER, fenster(&[gegenkonto(JAN_15, 20_000, "KONTO_9")]));
    let zweiter_import = api()
        .with_own_accounts(eigene_konten(&[("KONTO_9", "spar")]))
        .with_window(FENSTER, fenster(&[gegenkonto(MAR_05, 20_000, "KONTO_9")]));

    erster_import.run(FENSTER).expect("erster Import");
    zweiter_import.run(FENSTER).expect("zweiter Import");

    erster_import.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 20_000)],
        fingerprints: vec![],
        counter_accounts: vec![eigen("spar")],
        categories: vec![],
    });
    zweiter_import.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 3, 0, 20_000)],
        fingerprints: vec![],
        // **Dieselbe** Einordnung wie im ersten Import — trotz anderen Monats und anderem Lauf.
        counter_accounts: vec![eigen("spar")],
        categories: vec![],
    });
}

#[test]
fn eine_eigen_menge_aus_einem_anderen_seed_bricht_den_lauf_sichtbar_ab() {
    // Der Kern der Absicherung: wäre die Eigen-Menge mit einem **anderen** Seed tokenisiert,
    // matchte kein einziges Token — und jede Umbuchung liefe **lautlos** als „fremd" durch. Genau
    // das wäre der stille Datenfehler, den niemand bemerkt: ein Bericht, der Umbuchungen als
    // Konsum ausweist, ohne dass irgendetwas fehlschlägt.
    //
    // Darum tragen Fenster und Config je einen **opaken Seed-Marker** — nicht den Seed, sondern
    // ein nicht umkehrbares Etikett seiner Identität. Stimmen sie nicht überein, bricht der Lauf
    // **terminal** ab, statt ein falsches Ergebnis abzulegen.
    let api = api()
        .with_own_accounts(eigene_konten_mit_marker("marker-b", &[("KONTO_9", "spar")]))
        .with_window(
            FENSTER,
            fenster_mit_marker("marker-a", &[gegenkonto(JAN_15, 20_000, "KONTO_9")]),
        );

    let fehler = api.run(FENSTER).expect_err("erwartet terminalen Fehler");

    assert_eq!(fehler, EvaluationError::SeedMismatch);
    // Die Begründung trägt **keinen** der beiden Marker — sie benennt die Lage, nicht die Werte.
    assert_eq!(
        fehler.to_string(),
        "own-accounts configuration and analysis window were tokenised under different seeds"
    );
    api.assert_nothing_stored();
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
            seed_marker: SEED_MARKER.to_string(),
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

// --- Das Kategorisierer-Orakel: nur für Unbekanntes (Issue 0044) ---------------

#[test]
fn eine_unbekannte_buchungsart_wird_angefragt_und_ihre_antwort_am_fingerabdruck_zugeordnet() {
    // Der Kern von ADR-0004: die **Domäne entscheidet**, wann sie fragt — das LLM ist ein Orakel
    // hinter einem Port, kein Pipeline-Schritt. Kennt sie diese Art nicht, geht **eine** Anfrage
    // hinaus, und die Antwort findet über den **Fingerabdruck** zurück, nie über den Text.
    let schluessel = fingerabdruck(HAENDLER);
    let api = api()
        .with_oracle(&[(&schluessel, "Spiel & Spaß")])
        .with_window(
            FENSTER,
            fenster(&[firma(JAN_15, 1_299, HAENDLER, "Netflix", None)]),
        );

    api.run(FENSTER).expect("die Auswertung des Fensters");

    api.assert_asked(vec![anfrage(&schluessel, "Netflix")]);
    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 1_299)],
        fingerprints: vec![schluessel.clone()],
        counter_accounts: vec![],
        categories: vec![kategorie(&schluessel, "Spiel & Spaß")],
    });
}

#[test]
fn eine_bereits_bekannte_buchungsart_erreicht_das_orakel_nicht() {
    // Der Lackmustest von ADR-0004: **die Domäne entscheidet, was sie fragen muss**. Kennt das
    // Kategorie-Register die Art schon, bleibt der Chat stumm — auch wenn das Orakel eine Antwort
    // parat hätte. Sie erneut zu erfragen stellte die gelernte Korrektur zur Disposition, die
    // gerade deshalb gelernt wurde, weil das Orakel danebenlag.
    let schluessel = fingerabdruck(HAENDLER);
    let api = api()
        .with_known_categories(&[&schluessel])
        .with_oracle(&[(&schluessel, "Spiel & Spaß")])
        .with_window(
            FENSTER,
            fenster(&[firma(JAN_15, 1_299, HAENDLER, "Netflix", None)]),
        );

    api.run(FENSTER).expect("die Auswertung des Fensters");

    api.assert_nothing_asked();
    // Und das Ergebnis trägt **kein** Etikett: was das Register führt, gehört ihm — es an jedes
    // Fenster zu heften hieße, die gelernte Wahrheit zu duplizieren.
    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 1_299)],
        fingerprints: vec![schluessel],
        counter_accounts: vec![],
        categories: vec![],
    });
}

#[test]
fn mehrere_buchungen_derselben_art_erzeugen_genau_eine_anfrage() {
    // Der Fingerabdruck steht für eine *Art*, nicht für die einzelne Buchung — also ist er auch die
    // Achse der **Deduplizierung**. Drei Buchungen desselben Händlers sind **eine** Frage; sonst
    // wüchse der Chat-Payload mit der Zahl der Buchungen statt mit der Zahl der Arten, und das
    // Orakel bekäme dieselbe Frage dreimal.
    let schluessel = fingerabdruck(HAENDLER);
    let api = api()
        .with_oracle(&[(&schluessel, "Spiel & Spaß")])
        .with_window(
            FENSTER,
            fenster(&[
                firma(JAN_15, 1_299, HAENDLER, "Netflix", None),
                firma(JAN_31, 1_299, HAENDLER, "Netflix", None),
                firma(FEB_29, 1_299, HAENDLER, "Netflix", None),
            ]),
        );

    api.run(FENSTER).expect("drei Buchungen derselben Art");

    api.assert_asked(vec![anfrage(&schluessel, "Netflix")]);
}

#[test]
fn zwei_arten_mit_demselben_text_bekommen_je_ihre_eigene_antwort() {
    // Der Beleg, dass die Antwort auf den **Fingerabdruck** schlüsselt und **nicht auf den Text**:
    // zwei verschiedene Firmen tragen hier denselben Namen, also denselben Anfrage-Text — und
    // bekommen trotzdem je ihr eigenes Etikett. Liefe die Zuordnung über den Text, wäre sie hier
    // mehrdeutig; über den Schlüssel ist sie es nicht, und ein Rückweg vom Etikett auf den Freitext
    // existiert gar nicht erst.
    let eine = fingerabdruck("DE11ZZZ00000000001");
    let andere = fingerabdruck("DE22ZZZ00000000002");
    let api = api()
        .with_oracle(&[(&eine, "Lebensmittel"), (&andere, "Mobilität")])
        .with_window(
            FENSTER,
            fenster(&[
                firma(JAN_15, 1_000, "DE11ZZZ00000000001", "Kolibri", None),
                firma(JAN_31, 2_000, "DE22ZZZ00000000002", "Kolibri", None),
            ]),
        );

    api.run(FENSTER).expect("zwei Arten mit demselben Text");

    // Zwei Anfragen mit **identischem** Text, aber verschiedenen Schlüsseln.
    api.assert_asked(vec![anfrage(&eine, "Kolibri"), anfrage(&andere, "Kolibri")]);
    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 3_000)],
        fingerprints: vec![eine.clone(), andere.clone()],
        counter_accounts: vec![],
        categories: vec![
            kategorie(&eine, "Lebensmittel"),
            kategorie(&andere, "Mobilität"),
        ],
    });
}

#[test]
fn die_anfrage_traegt_weder_kontotoken_noch_pseudonym_noch_seed() {
    // **Datenschutz am einzigen Punkt, an dem Daten die Maschine verlassen.** Die Buchung führt ein
    // Gegenkonto-Token; die Anfrage trägt es nicht — sie hat kein Feld dafür. Was hinausgeht, ist
    // der Schlüssel (ein Derivat der öffentlichen Gläubiger-ID) und der obfuskierte Text, sonst
    // nichts. Die „kein `Serialize`"-Sperre auf dem Egress-DTO greift zur **Compile-Zeit**; dieser
    // Test hält den **Inhalt** fest, den sie schützt.
    let schluessel = fingerabdruck(HAENDLER);
    let api = api()
        .with_oracle(&[(&schluessel, "Lebensmittel")])
        .with_window(
            FENSTER,
            fenster(&[WindowBookingRow {
                counter_account: Some("KONTO_9".to_string()),
                ..firma(JAN_15, 4_200, HAENDLER, "Edeka", Some("Sagt Danke"))
            }]),
        );

    api.run(FENSTER).expect("eine Buchung mit Gegenkonto");

    api.assert_asked(vec![anfrage(&schluessel, "Edeka Sagt Danke")]);
}

#[test]
fn eine_buchung_ohne_kategorie_signal_wird_nicht_angefragt() {
    // Eine Entgeltzeile hat keine Gegenpartei und damit keinen Fingerabdruck; eine Firma ohne Namen
    // und ohne Verwendungszweck hat einen Schlüssel, aber **nichts zu deuten**. Beide Male gibt es
    // nichts zu fragen — und eine leere Frage zu stellen wäre Egress ohne Erkenntnis.
    let api = api().with_window(
        FENSTER,
        fenster(&[
            ausgang(JAN_15, 500),
            firma_ohne_namen(JAN_31, 900, HAENDLER, None),
        ]),
    );

    api.run(FENSTER).expect("nichts zu fragen");

    api.assert_nothing_asked();
}

#[test]
fn ohne_etwas_zu_fragen_wird_auch_das_register_nicht_befragt() {
    // Gibt es **keine** Anfrage, gibt es auch nichts abzugleichen — dann darf das Register gar nicht
    // erst angefasst werden. Der Beleg dafür ist ein **ausgefallenes** Register: ein Fenster ohne
    // Kategorie-Signal läuft trotzdem durch. Andernfalls beendete eine Auskunft, auf die es nicht
    // ankam, einen Lauf, der nichts von ihr wollte — ein Fehlschlag ohne Ursache in der Sache.
    let api = api()
        .with_window(FENSTER, fenster(&[ausgang(JAN_15, 500)]))
        .with_failing_registry("Register-Tabelle gesperrt");

    let ergebnis = api.run(FENSTER);

    assert_eq!(ergebnis, Ok(()));
    api.assert_nothing_asked();
    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 500)],
        fingerprints: vec![],
        counter_accounts: vec![],
        categories: vec![],
    });
}

#[test]
fn ein_etikett_zu_einem_nie_gefragten_schluessel_wird_verworfen() {
    // Der Fingerabdruck ist **Domänenwissen** (ADR-0004), keine Absprache über Zeichenketten. Ein
    // Orakel, das ein Etikett zu einem Schlüssel schickt, nach dem niemand gefragt hat, behauptet
    // etwas — es antwortet nicht. Die Behauptung fällt weg; sie **beendet** den Lauf aber nicht,
    // denn die Auswertung darf nicht von der Auskunftsfreude eines LLM abhängen.
    let schluessel = fingerabdruck(HAENDLER);
    let api = api()
        .with_oracle(&[(&schluessel, "Spiel & Spaß")])
        .with_rogue_oracle_answers(&[("COMPANY:NIE-GEFRAGT", "Erfundenes")])
        .with_window(
            FENSTER,
            fenster(&[firma(JAN_15, 1_299, HAENDLER, "Netflix", None)]),
        );

    api.run(FENSTER).expect("die Auswertung des Fensters");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 1_299)],
        fingerprints: vec![schluessel.clone()],
        counter_accounts: vec![],
        // Nur das erfragte Etikett — der erfundene Schlüssel hat es nicht hereingeschafft.
        categories: vec![kategorie(&schluessel, "Spiel & Spaß")],
    });
}

#[test]
fn zwei_widersprechende_etiketten_zu_einer_art_ergeben_gar_kein_etikett() {
    // Das Ergebnis ist eine **Abbildung** Fingerabdruck → Kategorie. Schickt das Orakel zu **einer**
    // Art zwei **verschiedene** Etiketten, hat es die Art nicht gedeutet, sondern sich
    // widersprochen — und aus einem Widerspruch eines auszuwählen hieße, eine Deutung zu erfinden,
    // die niemand vorgenommen hat. Sie landete anschließend als *gelernte* Kategorie im Register.
    //
    // Also: kein Etikett. Die Art bleibt unbekannt und darf im nächsten Lauf erneut gefragt werden —
    // lieber gar kein Ergebnis als ein falsches, das sich richtig liest (dieselbe Linie wie beim
    // Seed-Mismatch). Terminal ist es **nicht**: die Auswertung hängt nicht an der Auskunftsfreude
    // eines LLM.
    let schluessel = fingerabdruck(HAENDLER);
    let api = api()
        .with_oracle(&[(&schluessel, "Lebensmittel")])
        .with_rogue_oracle_answers(&[(&schluessel, "Mobilität")])
        .with_window(
            FENSTER,
            fenster(&[firma(JAN_15, 1_299, HAENDLER, "Netflix", None)]),
        );

    api.run(FENSTER).expect("ein widerspruechliches Orakel");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 1_299)],
        fingerprints: vec![schluessel],
        counter_accounts: vec![],
        categories: vec![],
    });
}

#[test]
fn dasselbe_etikett_zweimal_bleibt_ein_etikett() {
    // Die Gegenprobe zum Widerspruch oben: **dieselbe** Deutung doppelt geschickt ist kein
    // Widerspruch, sondern eine Wiederholung. Sie fällt zu **einem** Eintrag zusammen und geht
    // nicht etwa mit verloren — sonst bestrafte die Regel oben eine bloß redundante Antwort.
    let schluessel = fingerabdruck(HAENDLER);
    let api = api()
        .with_oracle(&[(&schluessel, "Spiel & Spaß")])
        .with_rogue_oracle_answers(&[(&schluessel, "Spiel & Spaß")])
        .with_window(
            FENSTER,
            fenster(&[firma(JAN_15, 1_299, HAENDLER, "Netflix", None)]),
        );

    api.run(FENSTER).expect("eine doppelte, gleiche Antwort");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 1_299)],
        fingerprints: vec![schluessel.clone()],
        counter_accounts: vec![],
        categories: vec![kategorie(&schluessel, "Spiel & Spaß")],
    });
}

#[test]
fn ein_fehlschlag_des_kategorie_registers_beendet_den_lauf_terminal() {
    // Terminal statt still weiterlaufen: hielte der Lauf bei ausgefallener Auskunft einfach alles
    // für unbekannt, ginge jede bereits gelernte Korrektur erneut ans Orakel — und dessen Antwort
    // überschriebe womöglich genau die Korrektur, die sie hervorgebracht hat. Kein Dead-Letter,
    // kein Retry im Slice (ADR-0012).
    let api = api()
        .with_window(
            FENSTER,
            fenster(&[firma(JAN_15, 1_299, HAENDLER, "Netflix", None)]),
        )
        .with_failing_registry("Register-Tabelle gesperrt");

    let ergebnis = api.run(FENSTER);

    assert_eq!(
        ergebnis,
        Err(EvaluationError::RegistryUnavailable {
            reason: "Register-Tabelle gesperrt".to_string()
        })
    );
    // Und **vor** dem Orakel: ohne zu wissen, was bekannt ist, darf nichts hinausgehen.
    api.assert_nothing_asked();
    api.assert_nothing_stored();
}

#[test]
fn ein_fehlschlag_des_orakels_beendet_den_lauf_terminal() {
    let api = api()
        .with_window(
            FENSTER,
            fenster(&[firma(JAN_15, 1_299, HAENDLER, "Netflix", None)]),
        )
        .with_failing_categorizer("Chat-Payload nicht zustellbar");

    let ergebnis = api.run(FENSTER);

    assert_eq!(
        ergebnis,
        Err(EvaluationError::CategorizerUnavailable {
            reason: "Chat-Payload nicht zustellbar".to_string()
        })
    );
    api.assert_nothing_stored();
}
