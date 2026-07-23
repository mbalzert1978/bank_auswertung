//! io-Integrationstest ([Issue 0020](../../../docs/issues/0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md),
//! **end-to-end real** seit [Issue 0053](../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md)):
//! ein `ReadTransactions`-Envelope läuft durch die generische Consumer-Maschinerie in `io` → den
//! echten `einlesen`-Slice → gegen **ausschließlich reale** Bausteine — echte CSV-Quelle
//! ([Issue 0051](../../../docs/issues/0051-einlesen-csv-camt-v8-quelle-real-hinter-source-seam.md)),
//! echte Geld-/Kalender-Dekodierung
//! ([Issue 0050](../../../docs/issues/0050-money-calendar-decode-seams-real.md)) und der echte
//! rusqlite-Ergebnis-Store hinter dem `StoreSeam` ([`EinlesenSqliteStore`](io::einlesen::EinlesenSqliteStore)).
//! **Kein Fake mehr** im einlesen-io-Pfad.
//!
//! Dieser Test prüft das über den **öffentlichen** Dispatch-Ausgang beobachtbare Verhalten: die
//! **terminalen, redigierten** Fehler jedes fehlbaren Schritts (Quelle / Präsenz / Decode) und die
//! Dispatch-Ebene (unbekannter `message_type`). Den committeten DB-Zustand (Zeile in der echten
//! Tabelle + `Ingested`-Zeiger, atomar) prüft der crate-interne Store-Test mit privatem
//! Verbindungszugriff (`crates/io/src/einlesen/store.rs`); alle diese Fehlerfälle steigen **vor** dem
//! Commit aus, sodass der Dispatch-Ausgang sie unmittelbar trägt.
//!
//! Der Fixture-Reader-Helfer und der Auslöser-Envelope liegen in den Fixtures nebenan
//! (`fixtures/einlesen/mod.rs`).

use io::Dispatcher;
use io::einlesen::{EinlesenSqliteStore, GermanCalendar, GermanMoney, IsoCurrencies};
use io::outbox::OutboxError;
use io::sqlite::SqliteOutbox;

#[path = "../fixtures/einlesen/mod.rs"]
mod fixtures;
use fixtures::{csv_source, read_transactions_envelope};

/// Verdrahtet den `einlesen`-Consumer über die gegebene Fixture-Datei und den **realen** Store und
/// dispatcht **einen** `ReadTransactions`-Envelope — die gemeinsame Real-Verdrahtung aller Fälle. Alle
/// hier geprüften Fälle failen **vor** dem Commit, darum genügt der direkte Dispatch (ohne Einreihen).
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test-Helfer: das assert! prüft die Registrierungs-Invariante, bevor dispatcht wird"
)]
#[expect(
    clippy::unwrap_in_result,
    reason = "Test-Helfer: das In-Memory-Setup kann nicht fehlschlagen; der Result-Rückgabewert ist \
              der Dispatch-Ausgang, den die Tests prüfen"
)]
fn dispatch_ueber(fixture: &str) -> Result<(), OutboxError> {
    let outbox = SqliteOutbox::open_in_memory().expect("In-Memory-Rückgrat öffnet");
    let store = EinlesenSqliteStore::new(outbox).expect("einlesen-Tabelle wird angelegt");
    let source = csv_source(fixture);
    let consumer = io::einlesen::consumer(
        &source,
        &GermanCalendar,
        &GermanMoney,
        &IsoCurrencies,
        store,
    );

    let mut dispatcher = Dispatcher::new();
    assert!(
        dispatcher
            .register(io::einlesen::CONSUMED_MESSAGE_TYPE, Box::new(consumer))
            .is_none(),
        "frische Registrierung darf keinen Consumer verdrängen"
    );

    dispatcher.dispatch(read_transactions_envelope())
}

// --- Quelle terminal: eine fehlende Datei ist ein terminaler, redigierter Beschaffungs-Fehler ----

#[test]
fn eine_fehlende_quell_datei_wird_terminal_und_redigiert() {
    let ergebnis = dispatch_ueber("existiert_nicht.csv");

    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason })
            if reason.contains("source unavailable")),
        "eine fehlende Quelldatei muss terminal (Warning) mit io-Diagnose failen: {ergebnis:?}"
    );
}

// --- Fehlendes Pflichtfeld: terminal + redigiert, nie ein Rohwert im Fehler --------------

#[test]
fn ein_fehlendes_pflichtfeld_wird_redigiert_terminal() {
    // Der Betrag der einzigen Zeile ist leer → die Adapter-ACL (`RawLine::try_from`) failt den
    // Batch terminal mit **nur** dem statischen Feldnamen, nie einem Rohwert.
    let leck = "DE89370400440532013000";

    let ergebnis = dispatch_ueber("pflichtfeld_fehlt.csv");

    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason })
            if reason.contains("amount") && !reason.contains(leck)),
        "erwartet redigiertes Warning mit Feldname, ohne Rohwert: {ergebnis:?}"
    );
}

// --- Undeutbarer Betrag: ein vorhandener, aber nicht dekodierbarer Wert failt terminal -----------

#[test]
fn ein_undeutbarer_betrag_wird_redigiert_terminal() {
    // Der Betrag ist vorhanden (Präsenz erfüllt), aber nicht dekodierbar → der Money-Seam gibt `None`,
    // das Aggregat mappt terminal auf `AmountInvalid`; der Grund benennt nur die Fehlerart, nie den
    // Rohwert.
    let leck = "kein-betrag";

    let ergebnis = dispatch_ueber("betrag_undeutbar.csv");

    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason })
            if reason.contains("not a decodable money amount") && !reason.contains(leck)),
        "erwartet redigiertes Warning zum undeutbaren Betrag, ohne Rohwert: {ergebnis:?}"
    );
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
