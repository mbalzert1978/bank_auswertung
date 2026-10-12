//! Black-Box-Spezifikation des **Fingerabdrucks** — der stabile Schlüssel einer *Art* von Buchung
//! (Issues 0041/0047). Wie `auswerten.rs` läuft alles ausschließlich über die public API des
//! Adapters (Fluent-`EvaluationTestApi` + DTOs), nie über Interna: der Schlüssel ist beobachtbares
//! Verhalten des Pro-Fenster-Handlers, kein Konstruktor-Test.
//!
//! Die Specs decken die drei Schlüssel-Quellen ab — Firma über die Gläubiger-ID, Durchleiter
//! zusätzlich über den Händler-Token des Verwendungszwecks, Privatperson über das seed-basierte
//! Pseudonym (ADR-0010) — und die Durchleiter-Erkennung vorrangig über die **Gläubiger-ID**
//! (Pflichtfeld), mit dem optionalen Namen nur als Auffangsignal.

use auswerten_adapter::WindowEvaluationRow;

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{
    FEB_01, FEB_29, FENSTER, JAN_15, JAN_31, MAR_05, api, fenster, firma, firma_ohne_namen, monat,
    person,
};

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
        transfers: vec![],
        fees: vec![],
        // **Ein** Eintrag, nicht zwei: die zwei Buchungen sind eine Art.
        fingerprints: vec!["COMPANY:DE98ZZZ09999999999".to_string()],
        counter_accounts: vec![],
        categories: vec![],
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
        transfers: vec![],
        fees: vec![],
        // **Zwei** Arten, nicht eine: der Händler-Token des Verwendungszwecks trennt sie.
        fingerprints: vec![
            "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:REWE-MARKT-IHR-EINKAUF".to_string(),
            "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:SPOTIFY-AB-IHR-EINKAUF".to_string(),
        ],
        counter_accounts: vec![],
        categories: vec![],
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
        transfers: vec![],
        fees: vec![],
        fingerprints: vec!["COMPANY:LU96ZZZ0000000000000000058|MERCHANT:REWE-MARKT".to_string()],
        counter_accounts: vec![],
        categories: vec![],
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
        transfers: vec![],
        fees: vec![],
        // Zwei Arten — erkannt über die **Gläubiger-ID**, die immer da ist.
        fingerprints: vec![
            "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:REWE-MARKT".to_string(),
            "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:SPOTIFY-AB".to_string(),
        ],
        counter_accounts: vec![],
        categories: vec![],
    });
}

#[test]
fn amazon_payments_ohne_gefuehrten_namen_verklumpt_seine_haendler_nicht() {
    // Amazon zieht Marketplace-Käufe als „AMAZON PAYMENTS EUROPE S.C.A." ein — ein Durchleiter wie
    // PayPal. Bislang trug ihn allein der Namens-Marker: eine **namenlose** Zeile (der Name ist von
    // `einlesen` an optional) fiele still zurück in den Klumpen. Die Gläubiger-ID stammt aus einem
    // echten Auszug (Issue 0047, HITL) — erst mit ihr greift die Erkennung auch ohne Namen.
    let api = api().with_window(
        FENSTER,
        fenster(&[
            firma_ohne_namen(
                JAN_15,
                4_235,
                "DE94ZZZ00000561653",
                Some("AMZN Mktp DE 302-1111111 Buchshop Nord"),
            ),
            firma_ohne_namen(
                JAN_31,
                999,
                "DE94ZZZ00000561653",
                Some("AMZN Mktp DE 305-2222222 Elektro Sued"),
            ),
        ]),
    );

    api.run(FENSTER).expect("namenloses Amazon Payments");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 5_234)],
        transfers: vec![],
        fees: vec![],
        // Zwei Arten, nicht eine — erkannt über die **Gläubiger-ID**, die immer da ist.
        fingerprints: vec![
            "COMPANY:DE94ZZZ00000561653|MERCHANT:AMZN-MKTP-DE-BUCHSHOP-NORD".to_string(),
            "COMPANY:DE94ZZZ00000561653|MERCHANT:AMZN-MKTP-DE-ELEKTRO-SUED".to_string(),
        ],
        counter_accounts: vec![],
        categories: vec![],
    });
}

#[test]
fn auch_amazon_digital_und_amazon_eu_tragen_den_haendler_token_ihres_zwecks() {
    // Amazon tritt im echten Auszug unter **drei** Firmierungen auf — neben Amazon Payments auch
    // „AMAZON DIGITAL GERMANY GMBH" und „AMAZON EU S.A R.L.". Deren Namen enthalten kein
    // „Payments", der Namens-Marker griffe nie; erst ihre Gläubiger-IDs machen sie zu Durchleitern.
    // Dass auch der Direkteinkauf nach dem Zweck aufgespalten wird (Prime vs. Kindle vs. Einkauf),
    // ist eine bewusste Analyse-Entscheidung (Issue 0047): der Zweck trägt hier die Art, die
    // Firmierung nur den Kanal.
    let api = api().with_window(
        FENSTER,
        fenster(&[
            firma_ohne_namen(
                JAN_15,
                1_099,
                "DE96ZZZ00000594888",
                Some("PRIME VIDEO 123-4567890"),
            ),
            firma_ohne_namen(
                JAN_31,
                2_499,
                "DE24ZZZ00000561652",
                Some("AMAZON.DE 028-3928372"),
            ),
        ]),
    );

    api.run(FENSTER).expect("Amazon Digital und Amazon EU");

    api.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 1, 0, 3_598)],
        transfers: vec![],
        fees: vec![],
        // Beide Schlüssel tragen den Händler-Token — ohne Durchleiter-Erkennung stünde hier nur der
        // nackte `COMPANY:`-Schlüssel und jede Firmierung wäre **ein** Klumpen.
        fingerprints: vec![
            "COMPANY:DE24ZZZ00000561652|MERCHANT:AMAZON-DE".to_string(),
            "COMPANY:DE96ZZZ00000594888|MERCHANT:PRIME-VIDEO".to_string(),
        ],
        counter_accounts: vec![],
        categories: vec![],
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
        transfers: vec![],
        fees: vec![],
        fingerprints: vec!["PERSON:PERSON_7".to_string()],
        counter_accounts: vec![],
        categories: vec![],
    });
    zweiter_import.assert_stored(WindowEvaluationRow {
        window_id: FENSTER.to_string(),
        currency: "EUR".to_string(),
        months: vec![monat(2024, 3, 0, 25_000)],
        transfers: vec![],
        fees: vec![],
        // **Derselbe** Schlüssel wie im ersten Import — trotz anderen Monats und anderem Lauf.
        fingerprints: vec!["PERSON:PERSON_7".to_string()],
        counter_accounts: vec![],
        categories: vec![],
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
        transfers: vec![],
        fees: vec![],
        // Ein **anderer** Schlüssel als `PERSON:PERSON_7` derselben Person oben.
        fingerprints: vec!["PERSON:PERSON_42".to_string()],
        counter_accounts: vec![],
        categories: vec![],
    });
}
