//! Die **test-lokalen** Fixtures/Helfer des `einlesen`-Integrationstests — der echte CSV-Reader auf
//! synthetischen Fixture-Dateien und der Auslöser-Envelope, die der nackte Test in `tests/` per
//! Modul-Include (`#[path = …] mod fixtures;`) hereinzieht.
//!
//! **Kein Fake mehr:** seit [Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md)
//! läuft der `einlesen`-Slice end-to-end **ohne einen einzigen Fake** — Quelle
//! ([Issue 0051](../../../../docs/issues/0051-einlesen-csv-camt-v8-quelle-real-hinter-source-seam.md)),
//! Geld/Kalender ([Issue 0050](../../../../docs/issues/0050-money-calendar-decode-seams-real.md)) und
//! der Ergebnis-Store ([`EinlesenSqliteStore`](io::einlesen::EinlesenSqliteStore) über `rusqlite`) sind
//! echt. Die frühere Fake-Arbeitseinheit ist entfallen; den committeten DB-Zustand prüft der
//! crate-interne Store-Test (privater Verbindungszugriff), dieser Integrationstest prüft das über den
//! **öffentlichen** Dispatch-Ausgang beobachtbare Verhalten (terminale, redigierte Fehler).

use io::outbox::OutboxMessage;

/// Baut den echten [`CsvCamtV8Source`](io::einlesen::CsvCamtV8Source) auf die benannte Fixture-Datei
/// unter `fixtures/einlesen/csv/` (Pfad relativ zum io-Crate). So läuft der Beschaffungs-Seam im Test
/// gegen den **echten** Reader (`fs` + `csv`), nicht gegen einen Fake.
pub(crate) fn csv_source(name: &str) -> io::einlesen::CsvCamtV8Source {
    let path = format!(
        "{}/fixtures/einlesen/csv/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    io::einlesen::CsvCamtV8Source::new(path)
}

/// Der `ReadTransactions`-Auslöser-Envelope: ein feldloser Trigger, dessen `partition_key` den
/// Auszug benennt (die Quelle steckt im echten Reader, nicht in der Nachricht).
pub(crate) fn read_transactions_envelope() -> OutboxMessage {
    OutboxMessage {
        id: "msg-1".to_string(),
        message_type: io::einlesen::CONSUMED_MESSAGE_TYPE.to_string(),
        partition_key: "auszug-juni".to_string(),
        content: String::new(),
        occurred_on_utc: 0,
        attempt: 0,
    }
}
