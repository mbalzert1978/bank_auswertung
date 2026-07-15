//! Die **spec-lokalen** Fixtures/Helfer des Obfuskierungs-Slice — Input-Builder, Expected-Builder,
//! Setup-/Zugriffs-Helfer und Test-Consts, die die nackten Tests in `tests/` per Modul-Include
//! (`#[path = …] mod fixtures;`) hereinziehen. Die Expected-Builder sind test-seitige
//! Erwartungsdaten (die erwartete Projektion eines Laufs) und gehören darum hierher, nicht in die
//! Adapter-`TestApi`.

use obfuskierung_adapter::{IngestedRow, ObfuscatedBooking, ObfuscationTestApi, Recipient};

pub const SEED: u64 = 42;
const EIGENE_IBAN: &str = "DE20123456780000001111";
const GEGEN_IBAN: &str = "DE93123456780000002222";

/// Eine vollständige, gültige einlesen-Zeile (Ausgabe an einen Privatpersonen-Empfänger ohne
/// Gläubiger-ID), wie sie im einlesen-Speicher liegt — die Fixtures leben in der Spec.
pub fn gültige_zeile() -> IngestedRow {
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

/// Ein Lauf-Setup mit gesetztem Seed und einer einzelnen unter „buchung-1" arrangierten Zeile —
/// die sonst wörtlich wiederholte Arrange-Kette der Ein-Zeilen-Szenarien, spec-lokal gebündelt.
pub fn api_mit_zeile(zeile: IngestedRow) -> ObfuscationTestApi {
    ObfuscationTestApi::new()
        .with_seed(SEED)
        .with_ingested("buchung-1", zeile)
}

/// Die erwartete obfuskierte Buchung zur [`gültige_zeile`] — die Token-Felder über die
/// deterministischen `expected_*`-Helfer der TestApi, alles andere Pass-Through/Quell-ID/Seed.
pub fn erwartete_buchung(api: &ObfuscationTestApi, source_id: &str) -> ObfuscatedBooking {
    ObfuscatedBooking {
        source_id: source_id.to_string(),
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
pub fn einzige_buchung(api: &ObfuscationTestApi) -> ObfuscatedBooking {
    let mut bookings = api.stored_bookings().into_iter();
    let buchung = bookings.next().expect("genau eine abgelegte Buchung");
    assert!(bookings.next().is_none(), "es darf nur eine Buchung geben");
    buchung
}
