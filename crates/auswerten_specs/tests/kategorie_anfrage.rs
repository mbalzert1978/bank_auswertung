//! Black-Box-Spezifikation der **Anfrage ans Orakel** — des einen Werts, der im Chat-Export die
//! Maschine verlässt ([Issue 0011](../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md)).
//! Wie die übrigen Specs ausschließlich über die Fluent-`EvaluationTestApi`.
//!
//! Die Domäne ordnet die Gegenpartei **einmal** ein (Durchleiter / Firma / Person / keine) und wählt
//! daraus den Text: beim Durchleiter **nur** den Zweck, bei einer Firma **nur** den Namen, bei einer
//! Person **nichts**. Auf jeden Text und auf den Händler-Token im Fingerabdruck läuft **dieselbe**
//! Egress-Wortregel: jedes Wort mit Ziffer oder `@` fällt, **bevor** zerlegt wird.

use auswerten_adapter::{EvaluationError, WindowBookingRow};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{FENSTER, JAN_15, JAN_31, anfrage, api, fenster, firma, firma_ohne_namen, person};

/// Die Gläubiger-ID von PayPal — ein Durchleiter.
const PAYPAL: &str = "LU96ZZZ0000000000000000058";
/// Eine erfundene, gewöhnliche Firma.
const STADTWERKE: &str = "DE98ZZZ09999999999";

fn ueber_paypal(zweck: Option<&str>) -> WindowBookingRow {
    firma(
        JAN_15,
        999,
        PAYPAL,
        "PayPal Europe S.a.r.l. et Cie S.C.A.",
        zweck,
    )
}

#[test]
fn beim_durchleiter_geht_der_zweck_hinaus_nicht_der_name() {
    let api = api().with_window(
        FENSTER,
        fenster(&[ueber_paypal(Some("PP.4711.PP Musikdienst Beispiel"))]),
    );

    api.run(FENSTER).expect("eine Zahlung über PayPal");

    // Der Name „PayPal …" sagt über die Kategorie nichts; der Händler steht im Zweck. Die wechselnde
    // Referenz `PP.4711.PP` trägt Ziffern und fällt als ganzes Wort — im Text wie im Token.
    api.assert_asked(vec![anfrage(
        &format!("COMPANY:{PAYPAL}|MERCHANT:MUSIKDIENST-BEISPIEL"),
        "Musikdienst Beispiel",
    )]);
}

#[test]
fn eine_e_mail_im_zweck_faellt_als_ganzes_wort_bevor_der_token_zerlegt_wird() {
    let api = api().with_window(
        FENSTER,
        fenster(&[ueber_paypal(Some(
            "PP.4711.PP max.mustermann@web.example Musikdienst Kd-Nr 12345 Tel 030-1234567",
        ))]),
    );

    api.run(FENSTER).expect("Zweck mit E-Mail und Nummern");

    // Zerlegte man zuerst an Satzzeichen, stünde `MAX-MUSTERMANN-WEB-EXAMPLE` im Fingerabdruck. So
    // fällt die Adresse an ihrem `@`, die Nummern an ihren Ziffern.
    api.assert_asked(vec![anfrage(
        &format!("COMPANY:{PAYPAL}|MERCHANT:MUSIKDIENST-KD-NR-TEL"),
        "Musikdienst Kd-Nr Tel",
    )]);
}

#[test]
fn ein_durchleiter_ohne_zweck_wird_nicht_angefragt() {
    let api = api().with_window(FENSTER, fenster(&[ueber_paypal(None)]));

    api.run(FENSTER).expect("PayPal ohne Zweck");

    // Der Name allein sagt nichts — eine Frage ohne Gegenstand wäre Egress ohne Erkenntnis.
    api.assert_nothing_asked();
}

#[test]
fn ein_durchleiter_dessen_zweck_nur_aus_nummern_besteht_wird_nicht_angefragt() {
    let api = api().with_window(
        FENSTER,
        fenster(&[ueber_paypal(Some("PP.4711.PP 1043287 2024-01-15"))]),
    );

    api.run(FENSTER).expect("PayPal mit reinem Nummern-Zweck");

    api.assert_nothing_asked();
}

#[test]
fn bei_einer_firma_geht_nur_der_name_hinaus_nicht_der_zweck() {
    let api = api().with_window(
        FENSTER,
        fenster(&[firma(
            JAN_15,
            8_500,
            STADTWERKE,
            "Stadtwerke Beispielstadt",
            Some("Abschlag Strom Kunde Erika Beispiel"),
        )]),
    );

    api.run(FENSTER).expect("eine Firma mit Zweck");

    // Im Zweck einer gewöhnlichen Firma steht oft der Kontoinhaber — er bleibt zu Hause.
    api.assert_asked(vec![anfrage(
        &format!("COMPANY:{STADTWERKE}"),
        "Stadtwerke Beispielstadt",
    )]);
}

#[test]
fn eine_firma_ohne_namen_wird_nicht_angefragt_auch_nicht_mit_zweck() {
    let api = api().with_window(
        FENSTER,
        fenster(&[firma_ohne_namen(
            JAN_15,
            8_500,
            STADTWERKE,
            Some("Abschlag Strom"),
        )]),
    );

    api.run(FENSTER).expect("eine Firma ohne Namen");

    // Kein Rückfall auf den Zweck: der ist bei einer gewöhnlichen Firma nicht ihr Kategorie-Signal.
    api.assert_nothing_asked();
}

#[test]
fn auch_aus_dem_firmennamen_faellt_jedes_wort_mit_ziffer() {
    let api = api().with_window(
        FENSTER,
        fenster(&[firma(JAN_15, 1_999, STADTWERKE, "1&1 Telecom", None)]),
    );

    api.run(FENSTER).expect("ein Firmenname mit Ziffer");

    api.assert_asked(vec![anfrage(&format!("COMPANY:{STADTWERKE}"), "Telecom")]);
}

#[test]
fn eine_person_wird_nie_angefragt_auch_nicht_mit_zweck() {
    let api = api().with_window(
        FENSTER,
        fenster(&[
            WindowBookingRow {
                purpose: Some("Miete Wohnung".to_string()),
                ..person(JAN_15, 75_000, "PERSON_7")
            },
            person(JAN_31, 2_000, "PERSON_8"),
        ]),
    );

    api.run(FENSTER).expect("zwei Überweisungen an Personen");

    // Ihr Fingerabdruck ist ein seed-abgeleitetes Pseudonym, ihr Zweck der riskanteste Freitext —
    // beides bleibt auf der Maschine. Preis: Personen (und Kartenzahlungen) bekommen per Chat kein
    // Etikett.
    api.assert_nothing_asked();
}

#[test]
fn eine_firmen_zeile_ohne_gueltige_glaeubiger_id_beendet_den_lauf_terminal_und_redigiert() {
    // So eine Zeile entsteht heute nicht mehr — `obfuskieren` parst die Gläubiger-ID. Sie kann aber
    // aus einer Datei älteren Stands stammen; dann ginge sie als `COMPANY:<Rohwert>` hinaus.
    for spalte in [
        "DE89370400440532013000",
        "Hans Vermieter",
        "DE00ZZZ00000000123",
    ] {
        let api = api().with_window(
            FENSTER,
            fenster(&[firma(JAN_15, 75_000, spalte, "Hans Vermieter", None)]),
        );

        let fehler = api.run(FENSTER);

        assert_eq!(fehler, Err(EvaluationError::InvalidCreditorId));
        assert!(!format!("{fehler:?}").contains(spalte));
        api.assert_nothing_stored();
        api.assert_nothing_asked();
    }
}
