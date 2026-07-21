//! io-Integrationstest ([Issue 0020](../../../docs/issues/0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md)):
//! ein `ReadTransactions`-Envelope läuft **end-to-end** durch die generische Consumer-Maschinerie
//! in `io` → den echten `einlesen`-Slice → bis zur publizierten „Ingested"-Nachricht. Getestet
//! wird die **io-Verdrahtung** (Envelope-Extraktion, `CommandDecoder`-Aufruf, atomarer
//! `UnitOfWork`-Commit, message_type-Auflösung, Dead-Letter-Taktik), nicht das Slice-Verhalten
//! (das prüft der `einlesen`-Spec).
//!
//! **Geld-/Kalender-Seam sind seit [Issue 0050](../../../docs/issues/0050-money-calendar-decode-seams-real.md)
//! die echten io-Adapter** ([`GermanMoney`](io::einlesen::GermanMoney) über `rust_decimal`,
//! [`GermanCalendar`](io::einlesen::GermanCalendar) über `chrono`) — so läuft das **komplette
//! Konstrukt gegen die echten Libs verdrahtet**. Quelle und Arbeitseinheit bleiben Fakes (der echte
//! SQLite-/`rusqlite`-/CSV-Teil folgt in Build-Order Schritt 5). Die deutsche Zahl-/Datum-**Mechanik
//! selbst** wird hier nicht durchdekliniert (das ist Lib-Sache, kein io-Unit-Test) — die bestehenden
//! Fälle üben sie am Happy Path (`-50,00`, `13.06.2024`) bzw. terminal (IBAN im Betragsfeld) mit.
//!
//! Die Fake-Quelle und die Fake-Arbeitseinheit liegen in den Fixtures nebenan
//! (`fixtures/einlesen/mod.rs`).

use io::Dispatcher;
use io::einlesen::{GermanCalendar, GermanMoney};
use io::outbox::OutboxError;

#[path = "../fixtures/einlesen/mod.rs"]
mod fixtures;
use fixtures::{FakeDb, FakeSource, gültiger_rohsatz, read_transactions_envelope};

// --- Happy Path: Envelope → OutboxConsumer → einlesen → publiziertes „Ingested" ---------

#[test]
fn ein_envelope_wird_end_to_end_verarbeitet_und_atomar_committet() {
    let db = FakeDb::default();
    let source = FakeSource {
        records: vec![gültiger_rohsatz()],
    };
    let consumer = io::einlesen::consumer(&source, &GermanCalendar, &GermanMoney, db.clone());

    let mut dispatcher = Dispatcher::new();
    assert!(
        dispatcher
            .register(io::einlesen::CONSUMED_MESSAGE_TYPE, Box::new(consumer))
            .is_none(),
        "frische Registrierung darf keinen Consumer verdrängen"
    );

    let ergebnis = dispatcher.dispatch(read_transactions_envelope());

    assert_eq!(ergebnis, Ok(()));
    // Atomar committet: genau eine Roh-Zeile **und** genau eine „Ingested"-Nachricht, deren Zeiger
    // sich mit der Speicher-ID deckt (Tabelle = Wahrheit, Message = Pointer).
    assert_eq!(db.committed_lines().len(), 1);
    assert_eq!(db.committed_message_types(), vec!["Ingested"]);
    assert_eq!(db.committed_partition_keys(), db.committed_line_ids());
}

// --- Terminaler Fehler: redigiertes Dead-Letter, nichts committet -----------------------

#[test]
fn ein_terminaler_fehler_wird_redigiert_dead_lettered() {
    // Spaltenverschub: eine IBAN im Betragsfeld → der Betrag ist nicht dekodierbar (terminal).
    let leck = "DE89370400440532013000";
    let mut rohsatz = gültiger_rohsatz();
    rohsatz.amount = Some(format!("-{leck}"));
    let db = FakeDb::default();
    let source = FakeSource {
        records: vec![rohsatz],
    };
    let consumer = io::einlesen::consumer(&source, &GermanCalendar, &GermanMoney, db.clone());

    let mut dispatcher = Dispatcher::new();
    assert!(
        dispatcher
            .register(io::einlesen::CONSUMED_MESSAGE_TYPE, Box::new(consumer))
            .is_none(),
        "frische Registrierung darf keinen Consumer verdrängen"
    );

    let ergebnis = dispatcher.dispatch(read_transactions_envelope());

    // Terminal → `Warning` über die **redigierte** `Display` des Domänen-Fehlers (nie der Rohwert).
    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason }) if !reason.contains(leck)),
        "erwartet redigiertes Dead-Letter: {ergebnis:?}"
    );
    // Nichts committet: die Arbeitseinheit fiel zurück (Rollback).
    assert!(db.committed_lines().is_empty());
    assert!(db.committed_message_types().is_empty());
}

// --- Dispatch: ein unbekannter message_type ist auf der Dispatch-Ebene unzustellbar -----

#[test]
fn ein_unbekannter_message_type_wird_critical() {
    let dispatcher = Dispatcher::new();

    let mut envelope = read_transactions_envelope();
    envelope.message_type = "Unbekannt".to_string();
    let ergebnis = dispatcher.dispatch(envelope);

    assert!(
        matches!(&ergebnis, Err(OutboxError::Critical { source, .. }) if source == "Unbekannt"),
        "erwartet Critical mit dem unbekannten Dispatch-Key: {ergebnis:?}"
    );
}
