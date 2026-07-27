//! Black-Box-Spezifikation des Obfuskierungs-Slice — ausschließlich über die public API des
//! Adapters (Fluent-`ObfuscationTestApi` + DTOs + der re-exportierte Domänen-`SourceError`), nie über
//! Interna. Seit dem Kernel-Flip ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md))
//! spezifiziert der Spec auf **Handler-Ebene** über den `application_kernel`-Vertrag: `run` fährt die
//! **echte** Kette (`ObfuscateCommandDecoder` → `ObfuscatingHandler`) gegen öffentliche Fakes und gibt
//! den **nackten Domänen-`SourceError`** zurück — **keine** `OutboxError`-Asserts mehr (die
//! Outbox-Mechanik prüft der io-Integrationstest).
//!
//! Der Slice konsumiert einen `Ingested`-**Zeiger** (nur die ID), **liest** das rohe Aggregat per ID
//! aus dem einlesen-Speicher, obfuskiert seed-basiert, legt die **ganze** obfuskierte Buchung in der
//! `obfuskiert`-Tabelle ab (die Persistenz vergibt die neue ID) und publiziert **genau diese neue ID**
//! als Zeiger. Ein terminaler Fehler (fehlende Zeile oder fehlendes Pflichtfeld) lässt den Handler mit
//! dem `SourceError` aussteigen; nichts wird abgelegt/publiziert.
//!
//! Erwartete Ergebnisse formuliert die Spec als **DTO** (Token-Felder über die `expected_*`-Helfer
//! der TestApi) und lässt sie die **TestApi validieren** (`assert_stored`/`assert_published`).

use obfuskierung_adapter::{ObfuscationTestApi, Recipient, SourceError};

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

    // Getrimmt, inhaltlich erhalten (Scrubbing vertagt) — `expected_text` trimmt gleichermaßen.
    api.assert_stored(erwartete_buchung(&api, "buchung-1"));
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
        matches!(fehler, SourceError::NotFound(_)),
        "erwartet terminalen NotFound: {fehler:?}"
    );
    assert_eq!(
        fehler.to_string(),
        "referenced booking not found in source: fehlt-noch"
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
        Err(SourceError::MissingMandatoryField { field: "currency" })
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

    // Ein Match statt zweier — wie beim `NotFound`-Spec: den Fehler EINMAL auspacken, dann Variante
    // und Wortlaut am gebundenen Wert prüfen (die opake `BookingId` baut die Spec nicht selbst nach).
    let fehler = ergebnis.expect_err("erwartet terminalen Fehler");
    assert!(
        matches!(fehler, SourceError::StorageUnavailable { .. }),
        "erwartet terminales StorageUnavailable: {fehler:?}"
    );
    assert_eq!(
        fehler.to_string(),
        "could not store obfuscated booking for source booking buchung-1: Tabelle gesperrt"
    );
    api.assert_nothing_emitted();
}
