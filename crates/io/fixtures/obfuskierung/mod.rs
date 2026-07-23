//! Die **test-lokalen** Fixtures/Helfer des `obfuskierung`-Integrationstests — Seam-Fakes, die
//! Fake-Arbeitseinheit, die Dispatcher-Verdrahtung und die Input-Builder, die der nackte Test in
//! `tests/` per Modul-Include (`#[path = …] mod fixtures;`) hereinzieht.
//!
//! **Atomarität als Fake:** die Arbeitseinheit ([`FakeUnit`]) **ist** die gestagete Ablage
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
use std::rc::Rc;

use io::Dispatcher;
use io::obfuskierung::UnitFactory;
use io::outbox::{OutboxCommand, OutboxMessage, OutboxPublisher, UnitOfWork};
use obfuskierung_adapter::{
    IngestedRow, IngestedSourceSeam, ObfuscatedBooking, ObfuscatedStoreSeam, ObfuscatorSeam,
    StoreUnavailable,
};

#[path = "../commit_failed.rs"]
mod commit_failed;
use commit_failed::CommitFailed;

const SEED: u64 = 42;

// --- Seam-Fakes (minimale Stubs) --------------------------------------------------------

/// Fake des einlesen-Lese-Seams: hält die arrangierten Roh-Zeilen je ID (spielt den vertagten
/// einlesen-Speicher). `read` liefert die Zeile zur ID oder `None` (absent = gebrochenes Invariant).
#[derive(Default)]
pub(crate) struct FakeSource {
    rows: RefCell<Vec<(String, IngestedRow)>>,
}

impl FakeSource {
    pub(crate) fn with(id: &str, row: IngestedRow) -> Self {
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
pub(crate) struct FakeObfuscator;

impl ObfuscatorSeam for FakeObfuscator {
    fn obfuscate(&self, seed: u64, plaintext: &str) -> String {
        format!("tok-{:x}", seed ^ plaintext.len() as u64)
    }
}

// --- Fake-Arbeitseinheit: Ports = die Unit, die committet -------------------------------

/// Die geteilte, committete Wahrheit hinter allen Arbeitseinheiten.
#[derive(Clone, Default)]
pub(crate) struct FakeDb {
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
    pub(crate) fn failing_commit() -> Self {
        Self {
            fail_commit: true,
            ..Self::default()
        }
    }

    pub(crate) fn committed_bookings(&self) -> Vec<ObfuscatedBooking> {
        self.committed
            .borrow()
            .bookings
            .iter()
            .map(|(_, booking)| booking.clone())
            .collect()
    }

    /// Die vom Store **neu vergebenen** IDs der abgelegten Buchungen — genau sie reisen als Zeiger.
    pub(crate) fn committed_assigned_ids(&self) -> Vec<String> {
        self.committed
            .borrow()
            .bookings
            .iter()
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Die **Quell-IDs** (Fremdschlüssel) der abgelegten Buchungen — zeigen auf die einlesen-Zeile.
    pub(crate) fn committed_source_ids(&self) -> Vec<String> {
        self.committed
            .borrow()
            .bookings
            .iter()
            .map(|(_, booking)| booking.source_id.clone())
            .collect()
    }

    pub(crate) fn committed_message_types(&self) -> Vec<String> {
        self.committed
            .borrow()
            .messages
            .iter()
            .map(|(message_type, _)| message_type.clone())
            .collect()
    }

    pub(crate) fn committed_partition_keys(&self) -> Vec<String> {
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

    fn begin(&self, _ack: io::MessageId<'_>) -> FakeUnit {
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
pub(crate) struct FakeUnit {
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

pub(crate) const EIGENE_IBAN: &str = "DE20123456780000001111";

pub(crate) fn gültige_zeile() -> IngestedRow {
    IngestedRow {
        ordering_account: EIGENE_IBAN.to_string(),
        counter_account: Some("DE93123456780000002222".to_string()),
        amount_minor_units: 1299,
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

pub(crate) fn ingested_envelope(pointer: &str) -> OutboxMessage {
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

pub(crate) fn dispatcher_mit<'a>(
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
