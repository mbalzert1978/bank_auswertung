//! Black-Box-Spezifikation des Obfuskierungs-Slice — ausschließlich über die public API des
//! Adapters (Fluent-`ObfuscationTestApi` + DTOs + der re-exportierte Domänen-`ObfuscationError`), nie über
//! Interna. Seit dem Kernel-Flip ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md))
//! spezifiziert der Spec auf **Handler-Ebene** über den `application_kernel`-Vertrag: `run` fährt die
//! **echte** Kette (`ObfuscateCommandDecoder` → `ObfuscatingHandler`) gegen öffentliche Fakes und gibt
//! den **nackten Domänen-`ObfuscationError`** zurück — **keine** `OutboxError`-Asserts mehr (die
//! Outbox-Mechanik prüft der io-Integrationstest).
//!
//! Der Slice konsumiert einen `Ingested`-**Zeiger** (nur die ID), **liest** das rohe Aggregat per ID
//! aus dem einlesen-Speicher, obfuskiert seed-basiert, legt die **ganze** obfuskierte Buchung in der
//! `obfuskiert`-Tabelle ab (die Persistenz vergibt die neue ID) und publiziert **genau diese neue ID**
//! als Zeiger. Ein terminaler Fehler (fehlende Zeile oder fehlendes Pflichtfeld) lässt den Handler mit
//! dem `ObfuscationError` aussteigen; nichts wird abgelegt/publiziert.
//!
//! Erwartete Ergebnisse formuliert die Spec als **DTO** (Token-Felder über die `expected_*`-Helfer
//! der TestApi) und lässt sie die **TestApi validieren** (`assert_stored`/`assert_published`).

use obfuskierung_adapter::{ObfuscationError, ObfuscationTestApi, Recipient};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{SEED, api_mit_zeile, einzige_buchung, erwartete_buchung, gültige_zeile};

// --- Happy Path: per ID lesen → obfuskiert ablegen → neue ID als Zeiger publizieren --

#[test]
fn zeile_wird_per_id_gelesen_obfuskiert_abgelegt_und_die_neue_id_als_zeiger_publiziert() {
    let api = api_mit_zeile(gültige_zeile());

    let ergebnis = api.run("buchung-1");

    assert_eq!(ergebnis, Ok(()));
    // Die ganze obfuskierte Buchung (Token + Pass-Through + Seed) liegt in der `obfuskiert`-Tabelle
    // und referenziert die Quell-Zeile nur per Fremdschlüssel (`source_id`) …
    api.assert_stored(erwartete_buchung(&api, "buchung-1"));
    // … und die **von der Ablage neu vergebene** ID reist als Zeiger durch die Outbox — nicht die
    // Quell-ID (Tabelle = Wahrheit, Message = Pointer).
    api.assert_published(&["obfuskiert-1"]);
}

// --- Seed-basierte Obfuskierung: deterministisch & unterscheidend --------------

#[test]
fn gleicher_klartext_und_seed_ergibt_dasselbe_konto_token() {
    let api = ObfuscationTestApi::new()
        .with_seed(SEED)
        .with_ingested("b1", gültige_zeile())
        .with_ingested("b2", gültige_zeile());
    api.run("b1").expect("gültige Zeile");
    api.run("b2").expect("gültige Zeile");

    let mut bookings = api.stored_bookings().into_iter();
    let erste = bookings.next().expect("erste Buchung");
    let zweite = bookings.next().expect("zweite Buchung");
    // Dieselbe IBAN + derselbe Seed → dasselbe Konto-Token (laufstabil, zustandslos).
    assert_eq!(erste.ordering_account, zweite.ordering_account);
}

// --- Fail-safe Firma vs. Privatperson (CONTEXT.md 113–116) ---------------------

#[test]
#[expect(
    clippy::panic,
    reason = "let-else-Bindung der erwarteten Variante: schlägt der Test laut fehl statt still zu überspringen"
)]
fn ohne_gläubiger_id_wird_der_empfänger_zur_pseudonymisierten_person() {
    // Die Person-Variante mit ihrem Pseudonym prüft `assert_stored` (Happy Path) bereits. Hier
    // zusätzlich die Redaktions-Invariante: der Klarname taucht im Token **nicht** auf.
    let api = api_mit_zeile(gültige_zeile()); // creditor_id: None
    api.run("buchung-1").expect("gültige Zeile");

    let buchung = einzige_buchung(&api);
    // Ein Match statt zweier: die Variante binden (sonst laut scheitern), dann am gebundenen
    // Pseudonym die Redaktion prüfen — keine klammheimlich stille `if let`-Extraktion.
    let Recipient::Person { pseudonym } = &buchung.recipient else {
        panic!(
            "erwartete eine pseudonymisierte Person, war {:?}",
            buchung.recipient
        );
    };
    assert!(!pseudonym.as_str().contains("Erika"));
    assert!(!pseudonym.as_str().contains("Mustermann"));
}

#[test]
fn mit_gläubiger_id_bleibt_der_firmenname_klartext_und_die_id_anker() {
    let mut zeile = gültige_zeile();
    zeile.recipient_name = Some("Stromanbieter AG".to_string());
    zeile.creditor_id = Some("DE00ZZZ00000000123".to_string());
    let api = api_mit_zeile(zeile);
    api.run("buchung-1").expect("gültige Zeile");

    // Firma → Name bleibt Klartext (Kategorie-Signal), Gläubiger-ID bleibt Anker.
    let mut erwartet = erwartete_buchung(&api, "buchung-1");
    erwartet.recipient = Recipient::Company {
        name: Some(api.expected_text("Stromanbieter AG")),
        creditor_id: "DE00ZZZ00000000123".to_string(),
    };
    api.assert_stored(erwartet);
}

#[test]
fn ohne_empfänger_und_ohne_gläubiger_id_gibt_es_keine_gegenpartei() {
    let mut zeile = gültige_zeile();
    zeile.recipient_name = None;
    zeile.creditor_id = None;
    let api = api_mit_zeile(zeile);
    api.run("buchung-1").expect("gültige Zeile");

    let mut erwartet = erwartete_buchung(&api, "buchung-1");
    erwartet.recipient = Recipient::Absent;
    api.assert_stored(erwartet);
}

// --- Freitext-Engpass ----------------------------------------------------------

#[test]
fn freitext_wird_getrimmt_durch_den_obfuskierten_text_engpass() {
    let mut zeile = gültige_zeile();
    zeile.purpose = Some("  Miete Juli  ".to_string());
    let api = api_mit_zeile(zeile);
    api.run("buchung-1").expect("gültige Zeile");

    // Getrimmt, inhaltlich erhalten — kein Identifikator darin, also nichts zu ersetzen.
    api.assert_stored(erwartete_buchung(&api, "buchung-1"));
}

// --- Scrubbing: eingebettete Identifikatoren verlassen den Engpass nicht ---------
//
// Je Identifikator-Typ ein eigener Fall. Alle Daten sind **erfunden**: die IBANs tragen die
// Prüfziffer `00` (strukturell unmöglich), die BICs die Fantasie-Bank `FIKT`, die Namen sind
// ausgedacht. Das Scrubbing ist heuristisch und nicht beweisbar vollständig — die Fälle unten belegen
// die einzelnen Regeln, nicht dass nichts mehr durchrutschen kann.

/// Fährt eine Buchung mit dem gegebenen Verwendungszweck (und den gegebenen bekannten Namen) und
/// prüft, dass genau der erwartete Zweck abgelegt wurde — alles andere wie in [`erwartete_buchung`].
fn assert_zweck_wird_abgelegt_als(bekannte_namen: &[&str], roher_zweck: &str, erwartet: &str) {
    let mut zeile = gültige_zeile();
    zeile.purpose = Some(roher_zweck.to_string());
    let api = bekannte_namen
        .iter()
        .fold(api_mit_zeile(zeile), |api, name| {
            api.with_known_person(name)
        });

    api.run("buchung-1").expect("gültige Zeile");

    let mut buchung = erwartete_buchung(&api, "buchung-1");
    buchung.purpose = Some(api.expected_text(erwartet));
    api.assert_stored(buchung);
}

#[test]
fn eine_iban_im_verwendungszweck_wird_ersetzt() {
    assert_zweck_wird_abgelegt_als(
        &[],
        "Rueckzahlung an DE00 1234 5678 9012 3456 78 danke",
        "Rueckzahlung an [IBAN] danke",
    );
}

#[test]
fn eine_zusammengeklebte_iban_wird_auch_ohne_wortgrenze_ersetzt() {
    // Fail-safe: die IBAN klebt am Wort davor und steht ohne Leerzeichen — sie fällt trotzdem.
    assert_zweck_wird_abgelegt_als(
        &[],
        "Konto IBANDE00123456789012345678 Miete",
        "Konto IBAN[IBAN] Miete",
    );
}

#[test]
fn eine_bic_im_verwendungszweck_wird_ersetzt() {
    assert_zweck_wird_abgelegt_als(
        &[],
        "Miete Juli an FIKTDEFF001 und FIKTDEFFXXX",
        "Miete Juli an [BIC] und [BIC]",
    );
}

#[test]
fn eine_auslaendische_bic_wird_hinter_dem_hinweis_bic_ersetzt() {
    // Ohne Ziffer und ohne Land `DE` erkennt die Form allein sie nicht — der Hinweis davor schon.
    assert_zweck_wird_abgelegt_als(&[], "Auslandsrate BIC: FIKTNLAA", "Auslandsrate BIC: [BIC]");
}

#[test]
fn grossgeschriebene_woerter_in_bic_form_bleiben_stehen() {
    // `PRIVATKONTO`, `KREDITKARTE` und `LASTSCHRIFT` haben genau die Form einer BIC (6 Buchstaben +
    // 5 Zeichen). Ohne Ziffer, ohne `DE` an Stelle 5–6 und ohne Hinweis bleiben sie stehen — an den
    // Buchungsarten hängt die Gebühren-Klassifikation stromabwärts.
    let mut zeile = gültige_zeile();
    zeile.purpose = Some("PRIVATKONTO KREDITKARTE".to_string());
    zeile.booking_text = Some("LASTSCHRIFT".to_string());
    let api = api_mit_zeile(zeile);

    api.run("buchung-1").expect("gültige Zeile");

    let mut buchung = erwartete_buchung(&api, "buchung-1");
    buchung.purpose = Some(api.expected_text("PRIVATKONTO KREDITKARTE"));
    buchung.booking_text = Some(api.expected_text("LASTSCHRIFT"));
    api.assert_stored(buchung);
}

#[test]
fn ein_bekannter_personen_name_wird_ersetzt() {
    assert_zweck_wird_abgelegt_als(
        &["Lisa Beispiel"],
        "Taschengeld Lisa Beispiel Juli",
        "Taschengeld [PERSON] Juli",
    );
}

#[test]
fn ein_bekannter_name_wird_auch_gross_geschrieben_und_in_bank_schreibweise_ersetzt() {
    // Banken schreiben gern „NACHNAME, VORNAME" in Großbuchstaben.
    assert_zweck_wird_abgelegt_als(
        &["Lisa Beispiel"],
        "TASCHENGELD BEISPIEL, LISA",
        "TASCHENGELD [PERSON]",
    );
}

#[test]
fn ein_bekannter_name_im_genitiv_wird_ersetzt() {
    assert_zweck_wird_abgelegt_als(
        &["Lisa Beispiel"],
        "Geschenk zu Lisa Beispiels Geburtstag",
        "Geschenk zu [PERSON] Geburtstag",
    );
}

#[test]
fn ein_bekannter_name_als_wortteil_bleibt_stehen() {
    // Nur ganze Wörter fallen: `Rose` steckt in `Rosenmontag`, ist dort aber kein Name.
    assert_zweck_wird_abgelegt_als(&["Rose"], "Rosenmontag Umzug", "Rosenmontag Umzug");
}

#[test]
fn ein_unbekannter_name_bleibt_stehen_das_scrubbing_ist_nicht_vollstaendig() {
    // Die ehrliche Grenze: was weder Empfänger war noch auf der Namensliste steht, kennt der
    // Engpass nicht. Dieser Fall hält fest, dass das Scrubbing keine Anonymisierung ist.
    assert_zweck_wird_abgelegt_als(
        &["Lisa Beispiel"],
        "Taschengeld Paul Erfunden",
        "Taschengeld Paul Erfunden",
    );
}

#[test]
fn eine_firma_ohne_gläubiger_id_auf_der_namensliste_wird_mitgeschwärzt() {
    // Der bekannte Preis von fail-safe: eine Firma ohne Gläubiger-ID gilt als Person, ihr Name
    // landet auf der Liste und fällt im Freitext mit.
    assert_zweck_wird_abgelegt_als(
        &["Gartenbau Fiktiv"],
        "Rechnung Gartenbau Fiktiv 2026",
        "Rechnung [PERSON] 2026",
    );
}

#[test]
fn der_firmenname_mit_gläubiger_id_laeuft_durch_denselben_engpass() {
    // Der Name einer sicheren Firma bleibt Klartext (Kategorie-Signal) — aber nicht roh: auch er
    // passiert den Engpass, ein darin stehender bekannter Personen-Name fällt.
    let mut zeile = gültige_zeile();
    zeile.recipient_name = Some("Praxis Lisa Beispiel".to_string());
    zeile.creditor_id = Some("DE00ZZZ00000000123".to_string());
    let api = api_mit_zeile(zeile).with_known_person("Lisa Beispiel");

    api.run("buchung-1").expect("gültige Zeile");

    let mut erwartet = erwartete_buchung(&api, "buchung-1");
    erwartet.recipient = Recipient::Company {
        name: Some(api.expected_text("Praxis [PERSON]")),
        creditor_id: "DE00ZZZ00000000123".to_string(),
    };
    api.assert_stored(erwartet);
}

#[test]
fn ein_fehlschlag_beim_lesen_der_namensliste_scheitert_terminal_mit_durchgereichter_diagnose() {
    // Ohne Namensliste wird nicht obfuskiert — lieber terminal scheitern als still zu wenig
    // scrubben. Der Fehler trägt nur die io-Diagnose, keinen Namen.
    let api = api_mit_zeile(gültige_zeile()).with_failing_persons("Namensliste nicht lesbar");

    let ergebnis = api.run("buchung-1");

    assert_eq!(
        ergebnis,
        Err(ObfuscationError::PersonsUnavailable {
            reason: "Namensliste nicht lesbar".to_string()
        })
    );
    api.assert_nothing_emitted();
}

// --- Absente ID: terminal, nichts emittiert -------------------------------------

#[test]
fn absente_id_scheitert_terminal_ohne_etwas_zu_emittieren() {
    let api = ObfuscationTestApi::new().with_seed(SEED); // kein Aggregat arrangiert

    let ergebnis = api.run("fehlt-noch");

    // Zur ID liegt keine Zeile vor — der Handler steigt terminal mit `NotFound` aus. Ein Match statt
    // zweier: den Fehler EINMAL auspacken, dann Variante und Wortlaut am gebundenen Wert prüfen — der
    // Grund trägt nur die opake ID redigiert (kein Rohwert).
    let fehler = ergebnis.expect_err("erwartet terminalen Fehler");
    assert!(
        matches!(fehler, ObfuscationError::NotFound(_)),
        "erwartet terminalen NotFound: {fehler:?}"
    );
    assert_eq!(
        fehler.to_string(),
        "referenced booking not found in source: fehlt-noch"
    );
    api.assert_nothing_emitted();
}

// --- Lese-Speicher unerreichbar: terminal, io-Diagnose durchgereicht ------------

#[test]
fn ein_fehlschlag_beim_lesen_scheitert_terminal_mit_durchgereichter_diagnose() {
    // Der Lese-Speicher ist unerreichbar (fehlende Roh-Tabelle, kaputte Datenbank) — kategorisch
    // etwas anderes, als dass zur ID nichts vorläge, und darum am Domänen-Fehler unterscheidbar.
    // Ganzwert-Vergleich wie bei `fenstern`/`auswerten`: er nagelt Variante **und** Nutzlast in
    // einer Zusicherung fest und beweist zugleich, dass der Fehler nichts weiter trägt — die
    // Diagnose ist reine Zugriffs-Mechanik, kein Rohwert der Zeile.
    let api = api_mit_zeile(gültige_zeile()).with_failing_source("Roh-Tabelle nicht vorhanden");

    let ergebnis = api.run("buchung-1");

    assert_eq!(
        ergebnis,
        Err(ObfuscationError::SourceUnavailable {
            reason: "Roh-Tabelle nicht vorhanden".to_string()
        })
    );
    api.assert_nothing_emitted();
}

// --- Vorhanden, aber Pflichtfeld leer: terminal (redigiert) ---------------------

#[test]
fn vorhandene_zeile_mit_leerem_pflichtfeld_scheitert_terminal() {
    // Die Zeile liegt im Speicher, aber ein Pflichtfeld (Währung) ist leer — terminal, redigiert
    // (nur der statische Feldname, nie ein Rohwert).
    let mut zeile = gültige_zeile();
    zeile.currency = String::new();
    let api = api_mit_zeile(zeile);

    let ergebnis = api.run("buchung-1");

    // Redigiert: der Grund trägt nur den statischen Feldnamen, nie einen Rohwert (keine IBAN, kein
    // Name); nichts wird abgelegt oder publiziert.
    assert_eq!(
        ergebnis,
        Err(ObfuscationError::MissingMandatoryField { field: "currency" })
    );
    api.assert_nothing_emitted();
}

// --- Ablage unerreichbar: terminal, io-Diagnose durchgereicht -------------------

#[test]
fn ein_fehlschlag_beim_ablegen_scheitert_terminal_mit_durchgereichter_diagnose() {
    // Die Zeile ist gültig, aber der Speicher ist unerreichbar. Der Fehlschlag ist terminal (kein
    // Retry im Slice) und trägt **beides**: die opake Quell-ID und die io-Diagnose — die darf an der
    // Adapter-Grenze nicht verfallen, sonst ist ein unerreichbarer Store nicht diagnostizierbar.
    let api = api_mit_zeile(gültige_zeile()).with_failing_store("Tabelle gesperrt");

    let ergebnis = api.run("buchung-1");

    // Ganzwert-Vergleich wie bei `fenstern`/`auswerten`: er nagelt Variante **und** Nutzlast in
    // einer Zusicherung fest und beweist zugleich, dass der Fehler nichts weiter trägt.
    assert_eq!(
        ergebnis,
        Err(ObfuscationError::StorageUnavailable {
            reason: "Tabelle gesperrt".to_string()
        })
    );
    api.assert_nothing_emitted();
}
