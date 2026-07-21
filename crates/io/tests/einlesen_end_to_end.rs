//! io-Integrationstest ([Issue 0020](../../../docs/issues/0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md),
//! Quelle real seit [Issue 0051](../../../docs/issues/0051-einlesen-csv-camt-v8-quelle-real-hinter-source-seam.md)):
//! ein `ReadTransactions`-Envelope läuft **end-to-end** durch die generische Consumer-Maschinerie
//! in `io` → den echten `einlesen`-Slice → bis zur publizierten „Ingested"-Nachricht. Getestet wird
//! die **io-Verdrahtung** (Envelope-Extraktion, `CommandDecoder`-Aufruf, atomarer
//! `UnitOfWork`-Commit, message_type-Auflösung, Dead-Letter-Taktik) **zusammen mit** der echten
//! Beschaffung: seit 0051 steht an der Stelle des früheren Quell-Fakes der echte
//! CSV-CAMT-v8-Reader ([`CsvCamtV8Source`](io::einlesen::CsvCamtV8Source)) auf **synthetischen
//! Fixture-Dateien** nach echtem Sparkassen-Header.
//!
//! **Geld-/Kalender-Seam sind seit [Issue 0050](../../../docs/issues/0050-money-calendar-decode-seams-real.md)
//! die echten io-Adapter** ([`GermanMoney`](io::einlesen::GermanMoney) über `rust_decimal`,
//! [`GermanCalendar`](io::einlesen::GermanCalendar) über `chrono`) — so läuft das **komplette
//! Konstrukt gegen die echten Libs verdrahtet**. Fake bleibt nur die Arbeitseinheit (der echte
//! SQLite-/`rusqlite`-Teil folgt in Build-Order Schritt 5). Verifiziert wird das Verhalten
//! **downstream über den einlesen-Bridge-Weg** gegen Fakes (Fixture → `RawRecord`s → `RawLine`s →
//! committete `Line`), **nicht** als isolierter Reader-Unit-Test.
//!
//! Die Fake-Arbeitseinheit, der Fixture-Reader-Helfer und die erwartete Zeile liegen in den
//! Fixtures nebenan (`fixtures/einlesen/mod.rs`).

use io::Dispatcher;
use io::einlesen::{GermanCalendar, GermanMoney};
use io::outbox::OutboxError;

#[path = "../fixtures/einlesen/mod.rs"]
mod fixtures;
use fixtures::{FakeDb, csv_source, erwartete_gueltige_zeile, read_transactions_envelope};

/// Registriert den `einlesen`-Consumer über die gegebene Fixture-Datei und dispatcht **einen**
/// `ReadTransactions`-Envelope — die gemeinsame Verdrahtung aller Fälle.
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test-Helfer: das assert! prüft die Registrierungs-Invariante, bevor dispatcht wird"
)]
fn dispatch_ueber(db: &FakeDb, fixture: &str) -> Result<(), OutboxError> {
    let source = csv_source(fixture);
    let consumer = io::einlesen::consumer(&source, &GermanCalendar, &GermanMoney, db.clone());

    let mut dispatcher = Dispatcher::new();
    assert!(
        dispatcher
            .register(io::einlesen::CONSUMED_MESSAGE_TYPE, Box::new(consumer))
            .is_none(),
        "frische Registrierung darf keinen Consumer verdrängen"
    );

    dispatcher.dispatch(read_transactions_envelope())
}

// --- Happy Path: Fixture-Datei → OutboxConsumer → einlesen → publiziertes „Ingested" -----

#[test]
fn eine_fixture_datei_wird_end_to_end_verarbeitet_und_atomar_committet() {
    let db = FakeDb::default();

    let ergebnis = dispatch_ueber(&db, "gueltig.csv");

    assert_eq!(ergebnis, Ok(()));
    // Die eine Fixture-Zeile wird feld-treu zur erwarteten `Line` projiziert (Fixture → erwarteter
    // `RawRecord` → `Line`) — ein `assert_eq!` über die ganze DTO beweist zugleich, dass **kein**
    // anderes Feld abweicht.
    assert_eq!(db.committed_lines(), vec![erwartete_gueltige_zeile()]);
    // Atomar committet: genau eine „Ingested"-Nachricht, deren Zeiger sich mit der Speicher-ID
    // deckt (Tabelle = Wahrheit, Message = Pointer).
    assert_eq!(db.committed_message_types(), vec!["Ingested"]);
    assert_eq!(db.committed_partition_keys(), db.committed_line_ids());
}

// --- Fehlendes Pflichtfeld: terminal + redigiert, nie ein Rohwert im Fehler --------------

#[test]
fn ein_fehlendes_pflichtfeld_wird_redigiert_dead_lettered() {
    // Der Betrag der einzigen Zeile ist leer → die Adapter-ACL (`RawLine::try_from`) failt den
    // Batch terminal mit **nur** dem statischen Feldnamen, nie einem Rohwert.
    let leck = "DE89370400440532013000";
    let db = FakeDb::default();

    let ergebnis = dispatch_ueber(&db, "pflichtfeld_fehlt.csv");

    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason })
            if reason.contains("amount") && !reason.contains(leck)),
        "erwartet redigiertes Dead-Letter mit Feldname, ohne Rohwert: {ergebnis:?}"
    );
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
