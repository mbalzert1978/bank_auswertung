//! io-Integrationstest ([Issue 0021](../../../docs/issues/0021-obfuskieren-end-to-end-ber-die-generische-outboxconsumer-in-io.md)):
//! ein `Ingested`-Envelope (Aggregat-Zeiger im `partition_key`) läuft **end-to-end** durch die
//! generische Consumer-Maschinerie in `io` → den echten `obfuskierung`-Slice → bis zur publizierten
//! „Obfuscated"-Nachricht. Getestet wird die **io-Verdrahtung** (Envelope-Extraktion,
//! `CommandDecoder`-Aufruf, atomarer `UnitOfWork`-Commit, message_type-Auflösung, Dead-Letter-/
//! Retry-Taktik), nicht das Slice-Verhalten (das prüft der `obfuskierung`-Spec). Alle Seams sind
//! Fakes; der echte SQLite-/`sqlx`-Teil bleibt draußen (Build-Order Schritt 5).
//!
//! **Atomarität als Fake:** die Arbeitseinheit (`FakeUnit`) **ist** die gestagete Ablage
//! (`ObfuscatedStoreSeam`) **und** die Outbox (`OutboxPublisher`); der `obfuskierung`-Handler wird von
//! `io` **aus ihr** gebaut, sodass Staging und Commit dieselbe Unit teilen. Sie hält die gestagete
//! Arbeit in eigenen `pending`-Puffern und flusht sie beim Commit in den geteilten `committed`-Sink;
//! wird sie ohne Commit fallengelassen (Handler-Fehler), verwirft das Fallenlassen der Unit ihre
//! `pending`-Arbeit (Rollback ohne Sonderfall). Das **Lesen** (`IngestedSourceSeam`) und der reine
//! `ObfuscatorSeam` liegen **außerhalb** der Transaktion — der Aufrufer reicht sie separat herein.
//!
//! Die Seam-Fakes sind **bewusst minimale Stubs** — kein Nachbau der echten Obfuskierung (die prüft
//! der `obfuskierung`-Spec hinter den Seams).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use io::Dispatcher;
use io::obfuskierung::UnitFactory;
use obfuskierung_adapter::{
    IngestedRow, IngestedSourceSeam, ObfuscatedBooking, ObfuscatedStoreSeam, ObfuscatorSeam,
    StoreUnavailable,
};
use shared_kernel::{OutboxCommand, OutboxError, OutboxMessage, OutboxPublisher, UnitOfWork};

const SEED: u64 = 42;

// --- Seam-Fakes (minimale Stubs) --------------------------------------------------------

/// Fake des einlesen-Lese-Seams: hält die arrangierten Roh-Zeilen je ID (spielt den vertagten
/// einlesen-Speicher). `read` liefert die Zeile zur ID oder `None` (absent = gebrochenes Invariant).
#[derive(Default)]
struct FakeSource {
    rows: RefCell<Vec<(String, IngestedRow)>>,
}

impl FakeSource {
    fn with(id: &str, row: IngestedRow) -> Self {
        let source = Self::default();
        source.rows.borrow_mut().push((id.to_string(), row));
        source
    }
}

impl IngestedSourceSeam for FakeSource {
    fn read(&self, id: &str) -> Option<IngestedRow> {
        self.rows
            .borrow()
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, row)| row.clone())
    }
}

/// Minimaler Obfuskator-Stub: ein deterministischer, nicht-leerer Ersatzwert ohne Klartext-Bezug —
/// genug, damit die Domäne ein Aggregat baut. Die echte Vergabe prüft der `obfuskierung`-Spec.
struct FakeObfuscator;

impl ObfuscatorSeam for FakeObfuscator {
    fn obfuscate(&self, seed: u64, plaintext: &str) -> String {
        format!("tok-{:x}", seed ^ plaintext.len() as u64)
    }
}

// --- Fake-Arbeitseinheit: Ports = die Unit, die committet -------------------------------

/// Ein Commit ist gescheitert (io-interner Retry-Ursprung). Trägt eine **Transaktions**-Diagnose,
/// nie einen Roh-Zeilenwert (Redaktions-Pflicht des Commit-Pfades).
#[derive(Debug)]
struct CommitFailed;

impl fmt::Display for CommitFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "commit fehlgeschlagen (broker weg)")
    }
}

impl std::error::Error for CommitFailed {}

/// Die geteilte, committete Wahrheit hinter allen Arbeitseinheiten.
#[derive(Clone, Default)]
struct FakeDb {
    committed: Rc<RefCell<Committed>>,
    fail_commit: bool,
}

#[derive(Default)]
struct Committed {
    /// Je abgelegter Buchung die vom Store **neu vergebene** ID und das DTO.
    bookings: Vec<(String, ObfuscatedBooking)>,
    messages: Vec<(String, String)>,
}

impl FakeDb {
    fn failing_commit() -> Self {
        Self {
            fail_commit: true,
            ..Self::default()
        }
    }

    fn committed_bookings(&self) -> Vec<ObfuscatedBooking> {
        self.committed
            .borrow()
            .bookings
            .iter()
            .map(|(_, booking)| booking.clone())
            .collect()
    }

    /// Die vom Store **neu vergebenen** IDs der abgelegten Buchungen — genau sie reisen als Zeiger.
    fn committed_assigned_ids(&self) -> Vec<String> {
        self.committed
            .borrow()
            .bookings
            .iter()
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Die **Quell-IDs** (Fremdschlüssel) der abgelegten Buchungen — zeigen auf die einlesen-Zeile.
    fn committed_source_ids(&self) -> Vec<String> {
        self.committed
            .borrow()
            .bookings
            .iter()
            .map(|(_, booking)| booking.source_id.clone())
            .collect()
    }

    fn committed_message_types(&self) -> Vec<String> {
        self.committed
            .borrow()
            .messages
            .iter()
            .map(|(message_type, _)| message_type.clone())
            .collect()
    }

    fn committed_partition_keys(&self) -> Vec<String> {
        self.committed
            .borrow()
            .messages
            .iter()
            .map(|(_, partition_key)| partition_key.clone())
            .collect()
    }
}

impl UnitFactory for FakeDb {
    type Unit = FakeUnit;

    fn begin(&self) -> FakeUnit {
        FakeUnit {
            committed: Rc::clone(&self.committed),
            pending: RefCell::new(Committed::default()),
            fail_commit: self.fail_commit,
        }
    }
}

/// Eine Arbeitseinheit: die gestagete Ablage **und** die Outbox in einem. `save`/`publish` stagen in
/// eigene `pending`-Puffer; `commit` flusht sie in den geteilten `committed`-Sink (oder scheitert,
/// wenn `fail_commit` arrangiert ist — der io-interne Retry-Ursprung). Ohne Commit (Handler-Fehler)
/// verwirft das Fallenlassen der Unit die `pending`-Arbeit — Rollback ohne Drop-Impl.
struct FakeUnit {
    committed: Rc<RefCell<Committed>>,
    pending: RefCell<Committed>,
    fail_commit: bool,
}

impl ObfuscatedStoreSeam for FakeUnit {
    fn save(&self, booking: &ObfuscatedBooking) -> Result<String, StoreUnavailable> {
        let mut pending = self.pending.borrow_mut();
        // Deterministische, je abgelegter Buchung eindeutige ID (der Fake spielt `INSERT … RETURNING id`).
        let id = format!(
            "obfuskiert-{}",
            self.committed.borrow().bookings.len() + pending.bookings.len() + 1
        );
        pending.bookings.push((id.clone(), booking.clone()));
        Ok(id)
    }
}

impl OutboxPublisher for FakeUnit {
    fn publish<T: OutboxCommand>(&self, command: T) {
        self.pending.borrow_mut().messages.push((
            T::MESSAGE_TYPE.to_string(),
            command.partition_key().to_string(),
        ));
    }
}

impl UnitOfWork for FakeUnit {
    type Error = CommitFailed;

    fn commit(self) -> Result<(), CommitFailed> {
        if self.fail_commit {
            return Err(CommitFailed);
        }
        let pending = self.pending.into_inner();
        let mut committed = self.committed.borrow_mut();
        committed.bookings.extend(pending.bookings);
        committed.messages.extend(pending.messages);
        Ok(())
    }
}

// --- Fixtures ---------------------------------------------------------------------------

const EIGENE_IBAN: &str = "DE20123456780000001111";

fn gültige_zeile() -> IngestedRow {
    IngestedRow {
        ordering_account: EIGENE_IBAN.to_string(),
        counter_account: Some("DE93123456780000002222".to_string()),
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

fn ingested_envelope(pointer: &str) -> OutboxMessage {
    OutboxMessage {
        // Die Envelope-`id` ist eine io-vergebene Message-UUID — bewusst **verschieden** vom
        // Aggregat-Zeiger im `partition_key`, damit der Test den Handshake über `partition_key`
        // erzwingt.
        id: format!("msg-uuid-{pointer}"),
        message_type: io::obfuskierung::CONSUMED_MESSAGE_TYPE.to_string(),
        partition_key: pointer.to_string(),
        content: String::new(),
        occurred_on_utc: 0,
        attempt: 0,
    }
}

fn dispatcher_mit<'a>(
    source: &'a FakeSource,
    obfuscator: &'a FakeObfuscator,
    db: FakeDb,
) -> Dispatcher<'a> {
    let consumer = io::obfuskierung::consumer(source, obfuscator, SEED, db);
    let mut dispatcher = Dispatcher::new();
    assert!(
        dispatcher
            .register(io::obfuskierung::CONSUMED_MESSAGE_TYPE, Box::new(consumer))
            .is_none(),
        "frische Registrierung darf keinen Consumer verdrängen"
    );
    dispatcher
}

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
