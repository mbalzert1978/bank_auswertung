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

use auswerten_adapter::{
    AnalysisWindowRow, CounterAccountRow, EvaluationError, WindowEvaluationRow,
};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{
    FEB_01, FEB_29, FENSTER, JAN_15, JAN_31, MAR_05, SEED_MARKER, api, ausgang, eigen,
    eigene_konten, eigene_konten_mit_marker, eingang, fenster, fenster_mit_marker, firma,
    firma_ohne_namen, gegenkonto, monat, person,
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
        counter_accounts: vec![],
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
        counter_accounts: vec![],
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
        counter_accounts: vec![],
    });
}

#[test]
fn ein_intermediaer_ohne_gefuehrten_namen_verklumpt_seine_haendler_nicht() {
    // Der Firmenname ist **optional** (`recipient_name` ist von `einlesen` an durchweg `Option`),
    // die Gläubiger-ID an einer Firma dagegen Pflicht. Hinge die Intermediär-Erkennung allein am
    // Namen, fiele sie bei einer namenlosen Zeile still aus — und genau der Klumpen, den dieses
    // Ticket verhindert, wäre zurück, ohne dass irgendetwas fehlschlüge.
    let api = api().with_window(
        FENSTER,
        fenster(&[
            firma_ohne_namen(
                JAN_15,
                4_235,
                "LU96ZZZ0000000000000000058",
                Some("PP.4711.PP . REWE Markt"),
            ),
            firma_ohne_namen(
                JAN_31,
                999,
                "LU96ZZZ0000000000000000058",
                Some("PP.4712.PP . Spotify AB"),
            ),
        ]),
    );

    api.run(FENSTER).expect("namenloser Intermediaer");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 5_234)],
        // Zwei Arten — erkannt über die **Gläubiger-ID**, die immer da ist.
        fingerprints: vec![
            "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:PP-PP-REWE-MARKT".to_string(),
            "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:PP-PP-SPOTIFY-AB".to_string(),
        ],
        counter_accounts: vec![],
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
        counter_accounts: vec![],
    });
    zweiter_import.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 3, 0, 25_000)],
        // **Derselbe** Schlüssel wie im ersten Import — trotz anderen Monats und anderem Lauf.
        fingerprints: vec!["PERSON:PERSON_7".to_string()],
        counter_accounts: vec![],
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
        counter_accounts: vec![],
    });
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
    });
    zweiter_import.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 3, 0, 20_000)],
        fingerprints: vec![],
        // **Dieselbe** Einordnung wie im ersten Import — trotz anderen Monats und anderem Lauf.
        counter_accounts: vec![eigen("spar")],
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
