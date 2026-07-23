//! io-Integrationstest ([Issue 0021](../../../docs/issues/0021-obfuskieren-end-to-end-ber-die-generische-outboxconsumer-in-io.md)):
//! ein `Ingested`-Envelope (Aggregat-Zeiger im `partition_key`) läuft **end-to-end** durch die
//! generische Consumer-Maschinerie in `io` → den echten `obfuskierung`-Slice → bis zur publizierten
//! „Obfuscated"-Nachricht. Getestet wird die **io-Verdrahtung** (Envelope-Extraktion,
//! `CommandDecoder`-Aufruf, atomarer `UnitOfWork`-Commit, message_type-Auflösung, Dead-Letter-/
//! Retry-Taktik), nicht das Slice-Verhalten (das prüft der `obfuskierung`-Spec). Alle Seams sind
//! Fakes; der echte SQLite-/`rusqlite`-Teil bleibt draußen (Build-Order Schritt 5).
//!
//! Die Seam-Fakes, die Fake-Arbeitseinheit und die Dispatcher-Verdrahtung liegen in den Fixtures
//! nebenan (`fixtures/obfuskierung/mod.rs`).

use io::outbox::OutboxError;

#[path = "../fixtures/obfuskierung/mod.rs"]
mod fixtures;
use fixtures::{
    EIGENE_IBAN, FakeDb, FakeObfuscator, FakeSource, dispatcher_mit, dispatcher_mit_seed,
    gültige_zeile, ingested_envelope,
};

// --- Happy Path: Envelope → OutboxConsumer → obfuskierung → publiziertes „Obfuscated" ---

#[test]
fn ein_envelope_wird_end_to_end_verarbeitet_und_atomar_committet() {
    let source = FakeSource::with("buchung-1", gültige_zeile());
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, &FakeObfuscator, db.clone());

    let ergebnis = dispatcher.dispatch(ingested_envelope("buchung-1"));

    assert_eq!(ergebnis, Ok(()));
    // Atomar committet: genau eine obfuskierte Buchung **und** genau eine „Obfuscated"-Nachricht,
    // deren Zeiger sich mit der vom Store **neu vergebenen** ID deckt (Tabelle = Wahrheit, Message =
    // Pointer) — nicht mit der Quell-ID.
    assert_eq!(db.committed_bookings().len(), 1);
    assert_eq!(db.committed_message_types(), vec!["Obfuscated"]);
    assert_eq!(db.committed_partition_keys(), db.committed_assigned_ids());
    assert_eq!(db.committed_partition_keys(), vec!["obfuskiert-1"]);
    // Die abgelegte Buchung referenziert die einlesen-Zeile nur per Fremdschlüssel.
    assert_eq!(db.committed_source_ids(), vec!["buchung-1"]);
}

// --- Terminaler Handler-Fehler: redigiertes Dead-Letter, nichts committet ---------------

#[test]
fn ein_malformed_datensatz_wird_redigiert_dead_lettered() {
    // Pflichtfeld leer (Währung) → `MissingMandatoryField` (terminal). Der Grund trägt nur den
    // Feldnamen, nie den Rohwert (die IBAN im selben Datensatz).
    let mut zeile = gültige_zeile();
    zeile.currency = String::new();
    let source = FakeSource::with("buchung-1", zeile);
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, &FakeObfuscator, db.clone());

    let ergebnis = dispatcher.dispatch(ingested_envelope("buchung-1"));

    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason })
            if reason.contains("currency") && !reason.contains(EIGENE_IBAN)),
        "erwartet redigiertes terminales Dead-Letter: {ergebnis:?}"
    );
    // Nichts committet: die Arbeitseinheit fiel zurück (Rollback).
    assert!(db.committed_bookings().is_empty());
    assert!(db.committed_message_types().is_empty());
}

// --- Absente Zeile: uniform terminal (gebrochenes Invariant), kein Retry ----------------

#[test]
fn eine_absente_zeile_wird_uniform_terminal_dead_lettered() {
    // Keine Zeile arrangiert: der Slice sieht den Zeiger, aber die Roh-Zeile fehlt — ein gebrochenes
    // Invariant. `io` spiegelt **jeden** Handler-Fehler uniform terminal (`Warning`), nicht Retry.
    let source = FakeSource::default();
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, &FakeObfuscator, db.clone());

    let ergebnis = dispatcher.dispatch(ingested_envelope("fehlt-noch"));

    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason }) if reason.contains("fehlt-noch")),
        "eine absente Zeile muss terminal (Warning) dead-lettern, nicht retried werden: {ergebnis:?}"
    );
    assert!(db.committed_bookings().is_empty());
    assert!(db.committed_message_types().is_empty());
}

// --- Commit-Fehler: der einzige Retry-Fall, nichts committet ----------------------------

#[test]
fn ein_commit_fehler_wird_als_einziger_retry_fall_gespiegelt() {
    let source = FakeSource::with("buchung-1", gültige_zeile());
    let db = FakeDb::failing_commit();
    let dispatcher = dispatcher_mit(&source, &FakeObfuscator, db.clone());

    let ergebnis = dispatcher.dispatch(ingested_envelope("buchung-1"));

    // Der Handler gelingt, aber der Commit scheitert → `Retry` (nicht `Warning`); nichts wird sichtbar
    // committet.
    assert!(
        matches!(&ergebnis, Err(OutboxError::Retry { reason }) if reason.contains("commit")),
        "ein Commit-Fehler ist der einzige io-interne Retry-Ursprung: {ergebnis:?}"
    );
    assert!(db.committed_bookings().is_empty());
    assert!(db.committed_message_types().is_empty());
}

// --- Blanker Zeiger: Decoder `None` → Critical auf Envelope-Ebene -----------------------

#[test]
fn ein_blanker_partition_key_wird_critical() {
    let source = FakeSource::with("buchung-1", gültige_zeile());
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, &FakeObfuscator, db.clone());

    // Ein blanker `partition_key` ist kein gültiger Aggregat-Zeiger → Decoder `None` → `Critical`
    // (Envelope-Ebene), nicht als Handler-Fehler durchgereicht.
    let ergebnis = dispatcher.dispatch(ingested_envelope("   "));

    assert!(
        matches!(&ergebnis, Err(OutboxError::Critical { source, .. })
            if source == io::obfuskierung::CONSUMED_MESSAGE_TYPE),
        "ein blanker Zeiger muss zu Critical auf Envelope-Ebene führen: {ergebnis:?}"
    );
    assert!(db.committed_bookings().is_empty());
}

// --- Config-Loader speist den Seed in den obfuskierung-Consumer (Issue 0059) -------------

#[test]
fn der_aus_der_toml_geladene_seed_wird_in_den_consumer_gereicht() {
    // Der Loader liest den Seed über den **echten Dateisystem-Pfad** `load` aus der Fixture-Datei;
    // genau dieser Wert geht — als `u64`-Wert, nicht als Datei — in den `obfuskierung`-Consumer. Der
    // Slice liest **keine** Datei. Beweisbar am committeten Aggregat: es persistiert den Seed, mit dem
    // es obfuskiert wurde (ADR-0010), also muss dieser exakt der geladene Wert sein. Der Pfad wird
    // crate-relativ über `CARGO_MANIFEST_DIR` gebaut (unabhängig vom Working Directory des Runners).
    let config_pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/obfuskierung_seed/valid.toml");
    let config =
        io::config::SeedConfig::load(&config_pfad).expect("gültige Seed-Config lädt über load");
    let source = FakeSource::with("buchung-1", gültige_zeile());
    let db = FakeDb::default();

    // Der geladene Seed (nicht die Fixture-Konstante) reist explizit über den zentralen Helfer in den
    // Consumer — dieselbe „frische Registrierung"-Zusicherung wie im Standard-Pfad, ohne Duplizierung.
    let dispatcher = dispatcher_mit_seed(&source, &FakeObfuscator, config.seed(), db.clone());

    assert_eq!(dispatcher.dispatch(ingested_envelope("buchung-1")), Ok(()));

    // Der geladene Seed reiste bis ins committete Aggregat — nicht eine hartverdrahtete Konstante.
    let bookings = db.committed_bookings();
    assert_eq!(bookings.len(), 1, "genau eine committete Buchung erwartet");
    let buchung = bookings
        .first()
        .expect("genau eine committete Buchung erwartet");
    assert_eq!(buchung.seed, config.seed());
}
