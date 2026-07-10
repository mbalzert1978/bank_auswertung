//! Black-Box-Spezifikation des Obfuskierungs-Slice — ausschließlich über die public API des
//! Adapters (Fluent-`ObfuscationTestApi` + DTOs + die `shared_kernel`-Outbox-Ergebnisform), nie
//! über Interna. Der Slice konsumiert einen `Ingested`-**Zeiger** (nur die ID), **liest** das rohe
//! Aggregat per ID aus dem einlesen-Speicher, obfuskiert seed-basiert, legt die **ganze** obfuskierte
//! Buchung in der `obfuskiert`-Tabelle ab und publiziert **nur** die ID als Zeiger.
//!
//! Erwartete Ergebnisse formuliert die Spec als **DTO** (Token-Felder über die `expected_*`-Helfer
//! der TestApi) und lässt sie die **TestApi validieren** (`assert_stored`/`assert_published`).

use obfuskierung_adapter::{IngestedRow, ObfuscatedBooking, ObfuscationTestApi, Recipient};
use shared_kernel::OutboxError;

const SEED: u64 = 42;
const EIGENE_IBAN: &str = "DE20123456780000001111";
const GEGEN_IBAN: &str = "DE93123456780000002222";

/// Eine vollständige, gültige einlesen-Zeile (Ausgabe an einen Privatpersonen-Empfänger ohne
/// Gläubiger-ID), wie sie im einlesen-Speicher liegt — die Fixtures leben in der Spec.
fn gültige_zeile() -> IngestedRow {
    IngestedRow {
        ordering_account: EIGENE_IBAN.to_string(),
        counter_account: Some(GEGEN_IBAN.to_string()),
        amount_cent: 1299,
        currency: "EUR".to_string(),
        direction: "Outgoing".to_string(),
        recipient_name: Some("Erika Mustermann".to_string()),
        creditor_id: None,
        booking_day: "2026-07-01".to_string(),
        value_date: "2026-07-02".to_string(),
        purpose: Some("Miete Juli".to_string()),
        booking_text: Some("DAUERAUFTRAG".to_string()),
    }
}

/// Die erwartete obfuskierte Buchung zur [`gültige_zeile`] — die Token-Felder über die
/// deterministischen `expected_*`-Helfer der TestApi, alles andere Pass-Through/ID/Seed.
fn erwartete_buchung(api: &ObfuscationTestApi, id: &str) -> ObfuscatedBooking {
    ObfuscatedBooking {
        id: id.to_string(),
        ordering_account: api.expected_account(EIGENE_IBAN),
        counter_account: Some(api.expected_account(GEGEN_IBAN)),
        amount_cent: 1299,
        currency: "EUR".to_string(),
        direction: "Outgoing".to_string(),
        recipient: Recipient::Person {
            pseudonym: api.expected_pseudonym("Erika Mustermann"),
        },
        booking_day: "2026-07-01".to_string(),
        value_date: "2026-07-02".to_string(),
        purpose: Some(api.expected_text("Miete Juli")),
        booking_text: Some(api.expected_text("DAUERAUFTRAG")),
        seed: SEED,
    }
}

/// Die **eine** obfuskierte Buchung eines Laufs — indexfreier Zugriff für Eigenschafts-Prüfungen.
fn einzige_buchung(api: &ObfuscationTestApi) -> ObfuscatedBooking {
    let mut bookings = api.stored_bookings().into_iter();
    let buchung = bookings.next().expect("genau eine abgelegte Buchung");
    assert!(bookings.next().is_none(), "es darf nur eine Buchung geben");
    buchung
}

// --- Happy Path: per ID lesen → obfuskiert ablegen → ID als Zeiger publizieren --

#[test]
fn zeile_wird_per_id_gelesen_obfuskiert_abgelegt_und_die_id_als_zeiger_publiziert() {
    let api = ObfuscationTestApi::new()
        .with_seed(SEED)
        .with_ingested("buchung-1", gültige_zeile());

    let ergebnis = api.run("buchung-1");

    assert_eq!(ergebnis, Ok(()));
    // Die ganze obfuskierte Buchung (Token + Pass-Through + Seed) liegt in der `obfuskiert`-Tabelle …
    api.assert_stored(erwartete_buchung(&api, "buchung-1"));
    // … und nur die ID reist als Zeiger durch die Outbox (Tabelle = Wahrheit, Message = Pointer).
    api.assert_published(&["buchung-1"]);
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
    assert_eq!(
        erste.ordering_account.as_str(),
        zweite.ordering_account.as_str()
    );
}

// --- Fail-safe Firma vs. Privatperson (CONTEXT.md 113–116) ---------------------

#[test]
fn ohne_gläubiger_id_wird_der_empfänger_zur_pseudonymisierten_person() {
    // Die Person-Variante mit ihrem Pseudonym prüft `assert_stored` (Happy Path) bereits. Hier
    // zusätzlich die Redaktions-Invariante: der Klarname taucht im Token **nicht** auf.
    let api = ObfuscationTestApi::new()
        .with_seed(SEED)
        .with_ingested("buchung-1", gültige_zeile()); // creditor_id: None
    api.run("buchung-1").expect("gültige Zeile");

    let buchung = einzige_buchung(&api);
    assert!(
        matches!(&buchung.recipient, Recipient::Person { .. }),
        "erwartete eine pseudonymisierte Person, war {:?}",
        buchung.recipient
    );
    if let Recipient::Person { pseudonym } = &buchung.recipient {
        assert!(!pseudonym.as_str().contains("Erika"));
        assert!(!pseudonym.as_str().contains("Mustermann"));
    }
}

#[test]
fn mit_gläubiger_id_bleibt_der_firmenname_klartext_und_die_id_anker() {
    let mut zeile = gültige_zeile();
    zeile.recipient_name = Some("Stromanbieter AG".to_string());
    zeile.creditor_id = Some("DE00ZZZ00000000123".to_string());
    let api = ObfuscationTestApi::new()
        .with_seed(SEED)
        .with_ingested("buchung-1", zeile);
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
    let api = ObfuscationTestApi::new()
        .with_seed(SEED)
        .with_ingested("buchung-1", zeile);
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
    let api = ObfuscationTestApi::new()
        .with_seed(SEED)
        .with_ingested("buchung-1", zeile);
    api.run("buchung-1").expect("gültige Zeile");

    // Getrimmt, inhaltlich erhalten (Scrubbing vertagt) — `expected_text` trimmt gleichermaßen.
    api.assert_stored(erwartete_buchung(&api, "buchung-1"));
}

// --- Unbekannte ID: transient, nichts emittiert --------------------------------

#[test]
fn unbekannte_id_wird_transient_zurückgestellt_ohne_etwas_zu_emittieren() {
    let api = ObfuscationTestApi::new().with_seed(SEED); // kein Aggregat arrangiert

    let ergebnis = api.run("fehlt-noch");

    // Zur ID liegt (noch) kein Aggregat vor → erneute Zustellung (die einlesen-Schreibung kann
    // nachhängen); der Grund trägt die opake ID (kein Rohwert).
    assert_eq!(
        ergebnis,
        Err(OutboxError::Retry {
            reason: "referenced booking not found in source: fehlt-noch".to_string(),
        })
    );
    api.assert_nothing_emitted();
}

// --- Vorhanden, aber Pflichtfeld leer: terminaler Dead-Letter (kein Poison-Retry) ---

#[test]
fn vorhandene_zeile_mit_leerem_pflichtfeld_wird_terminal_dead_lettert() {
    // Die Zeile liegt im Speicher (also **nicht** „abwesend"), aber ein Pflichtfeld (Währung) ist
    // leer — das kann kein erneutes Zustellen heilen. Muss terminal dead-lettern (`Warning`), nicht
    // endlos zurückgestellt werden (Poison-Message).
    let mut zeile = gültige_zeile();
    zeile.currency = String::new();
    let api = ObfuscationTestApi::new()
        .with_seed(SEED)
        .with_ingested("buchung-1", zeile);

    let ergebnis = api.run("buchung-1");

    // Redigiert: der Grund trägt nur den statischen Feldnamen, nie einen Rohwert (keine IBAN, kein
    // Name); nichts wird abgelegt oder publiziert.
    assert_eq!(
        ergebnis,
        Err(OutboxError::Warning {
            reason: "mandatory field missing in source booking: currency".to_string(),
        })
    );
    api.assert_nothing_emitted();
}
