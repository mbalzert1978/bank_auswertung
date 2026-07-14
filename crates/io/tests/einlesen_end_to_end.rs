//! io-Integrationstest ([Issue 0020](../../../docs/issues/0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md)):
//! ein `ReadTransactions`-Envelope läuft **end-to-end** durch die generische Consumer-Maschinerie
//! in `io` → den echten `einlesen`-Slice → bis zur publizierten „Ingested"-Nachricht. Getestet
//! wird die **io-Verdrahtung** (Envelope-Extraktion, `CommandDecoder`-Aufruf, `UnitOfWork`-Commit,
//! message_type-Auflösung, Dead-Letter-Taktik), nicht das Slice-Verhalten (das prüft der
//! `einlesen`-Spec). Alle Seams sind Fakes; der echte SQLite-/`sqlx`-Teil bleibt draußen
//! (Build-Order Schritt 5).
//!
//! **Atomarität als Fake:** die Store-/Publish-Seams *stagen* in `pending`-Puffer; die
//! `UnitOfWork` flusht sie beim Commit nach `committed` und verwirft nicht-committete Arbeit beim
//! Fallenlassen (Rollback). So beweist der Test, dass bei Erfolg **beides** atomar committet und
//! bei einem terminalen Fehler **nichts**.

use std::cell::RefCell;
use std::convert::Infallible;
use std::rc::Rc;

use einlesen_adapter::{
    CalendarDay, CalendarSeam, DecodedAmount, Line, MoneySeam, RawRecord, Sign, SourceSeam,
    SourceUnavailable, StoreSeam,
};
use io::{Dispatcher, PublishingPort, UnitOfWorkFactory};
use shared_kernel::{OutboxCommand, OutboxError, OutboxMessage, OutboxPublisher, UnitOfWork};

// --- Fakes der io-Seams -----------------------------------------------------------------

/// Fake-Quelle: liefert die arrangierten Roh-Zeilen (die Beschaffung gelingt).
struct FakeSource {
    records: Vec<RawRecord>,
}

impl SourceSeam for FakeSource {
    fn fetch(&self) -> Result<Vec<RawRecord>, SourceUnavailable> {
        Ok(self.records.clone())
    }
}

/// Fake-Kalender: deutet „TT.MM.JJJJ" (grobe Bereichsprüfung); ungültig → `None`.
struct FakeCalendar;

impl CalendarSeam for FakeCalendar {
    fn parse(&self, raw: &str) -> Option<CalendarDay> {
        let mut parts = raw.split('.');
        let day: u8 = parts.next()?.parse().ok()?;
        let month: u8 = parts.next()?.parse().ok()?;
        let year: u16 = parts.next()?.parse().ok()?;
        ((1..=12).contains(&month) && (1..=31).contains(&day)).then_some(CalendarDay {
            year,
            month,
            day,
        })
    }
}

/// Fake-Geld: streift das Vorzeichen ab, parst die Restziffern naiv; ohne Vorzeichen oder bei
/// Nicht-Ziffern → `None` (die Domäne sieht dann `AmountInvalid`).
struct FakeMoney;

impl MoneySeam for FakeMoney {
    fn decode(&self, raw: &str) -> Option<DecodedAmount> {
        let (sign, digits) = raw
            .strip_prefix('-')
            .map(|rest| (Sign::Negative, rest))
            .or_else(|| raw.strip_prefix('+').map(|rest| (Sign::Positive, rest)))?;
        let (euro, cent) = digits.split_once(',').unwrap_or((digits, ""));
        let euro: u64 = euro.replace('.', "").parse().ok()?;
        let cent: u64 = if cent.is_empty() {
            0
        } else {
            cent.parse().ok()?
        };
        Some(DecodedAmount {
            cent: euro.checked_mul(100)?.checked_add(cent)?,
            sign,
        })
    }
}

/// Der gemeinsame Fake-Speicher hinter Store-Seam, `OutboxPublisher` **und**
/// `UnitOfWorkFactory`: die drei teilen dieselben `pending`/`committed`-Puffer (Rc), sodass eine
/// [`FakeUnit`] die gestagete Store- **und** Publish-Arbeit gemeinsam committet — die eine
/// Arbeitseinheit des Outbox-Patterns.
#[derive(Clone, Default)]
struct FakeDb {
    state: Rc<RefCell<DbState>>,
}

#[derive(Default)]
struct DbState {
    next_id: u32,
    pending_lines: Vec<(String, Line)>,
    committed_lines: Vec<(String, Line)>,
    pending_messages: Vec<(String, String)>,
    committed_messages: Vec<(String, String)>,
}

impl FakeDb {
    fn committed_lines(&self) -> Vec<Line> {
        self.state
            .borrow()
            .committed_lines
            .iter()
            .map(|(_, line)| line.clone())
            .collect()
    }

    fn committed_line_ids(&self) -> Vec<String> {
        self.state
            .borrow()
            .committed_lines
            .iter()
            .map(|(id, _)| id.clone())
            .collect()
    }

    fn committed_message_types(&self) -> Vec<String> {
        self.state
            .borrow()
            .committed_messages
            .iter()
            .map(|(message_type, _)| message_type.clone())
            .collect()
    }

    fn committed_partition_keys(&self) -> Vec<String> {
        self.state
            .borrow()
            .committed_messages
            .iter()
            .map(|(_, partition_key)| partition_key.clone())
            .collect()
    }
}

impl StoreSeam for FakeDb {
    fn store(&self, line: &Line) -> String {
        let mut state = self.state.borrow_mut();
        state.next_id += 1;
        let id = format!("ingested-{}", state.next_id);
        state.pending_lines.push((id.clone(), line.clone()));
        id
    }
}

impl OutboxPublisher for FakeDb {
    fn publish<T: OutboxCommand>(&self, command: T) {
        self.state.borrow_mut().pending_messages.push((
            T::MESSAGE_TYPE.to_string(),
            command.partition_key().to_string(),
        ));
    }
}

impl UnitOfWorkFactory for FakeDb {
    type Unit = FakeUnit;

    fn begin(&self) -> FakeUnit {
        FakeUnit {
            state: Rc::clone(&self.state),
            committed: false,
        }
    }
}

/// Die eine Arbeitseinheit: `commit` flusht `pending` → `committed` (Store **und** Publish
/// gemeinsam); wird sie ohne Commit fallengelassen (Handler-Fehler), verwirft `Drop` die
/// gestagete Arbeit (Rollback).
struct FakeUnit {
    state: Rc<RefCell<DbState>>,
    committed: bool,
}

impl UnitOfWork for FakeUnit {
    type Error = Infallible;

    fn commit(mut self) -> Result<(), Infallible> {
        let mut state = self.state.borrow_mut();
        let staged_lines = std::mem::take(&mut state.pending_lines);
        state.committed_lines.extend(staged_lines);
        let staged_messages = std::mem::take(&mut state.pending_messages);
        state.committed_messages.extend(staged_messages);
        drop(state);
        self.committed = true;
        Ok(())
    }
}

impl Drop for FakeUnit {
    fn drop(&mut self) {
        if !self.committed {
            let mut state = self.state.borrow_mut();
            state.pending_lines.clear();
            state.pending_messages.clear();
        }
    }
}

// --- Fixtures ---------------------------------------------------------------------------

fn gültiger_rohsatz() -> RawRecord {
    RawRecord {
        ordering_account: Some("DE89370400440532013000".to_string()),
        booking_day: Some("13.06.2024".to_string()),
        value_date: Some("14.06.2024".to_string()),
        amount: Some("-50,00".to_string()),
        currency: Some("EUR".to_string()),
        recipient_name: Some("EDEKA SAGT DANKE".to_string()),
        creditor_id: Some("DE98ZZZ09999999999".to_string()),
        counter_account: Some("DE11520513735120710131".to_string()),
        purpose: Some("Einkauf".to_string()),
        booking_text: Some("LASTSCHRIFT".to_string()),
    }
}

fn read_transactions_envelope() -> OutboxMessage {
    OutboxMessage {
        id: "msg-1".to_string(),
        message_type: io::einlesen::CONSUMED_MESSAGE_TYPE.to_string(),
        partition_key: "auszug-juni".to_string(),
        content: String::new(),
        occurred_on_utc: 0,
        attempt: 0,
    }
}

// --- Happy Path: Envelope → OutboxConsumer → einlesen → publiziertes „Ingested" ---------

#[test]
fn ein_envelope_wird_end_to_end_verarbeitet_und_atomar_committet() {
    let db = FakeDb::default();
    let source = FakeSource {
        records: vec![gültiger_rohsatz()],
    };
    let calendar = FakeCalendar;
    let money = FakeMoney;
    let publishing = PublishingPort::new(&db);
    let consumer = io::einlesen::consumer(&source, &calendar, &money, &db, &publishing, db.clone());

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(io::einlesen::CONSUMED_MESSAGE_TYPE, Box::new(consumer));

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
    let calendar = FakeCalendar;
    let money = FakeMoney;
    let publishing = PublishingPort::new(&db);
    let consumer = io::einlesen::consumer(&source, &calendar, &money, &db, &publishing, db.clone());

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(io::einlesen::CONSUMED_MESSAGE_TYPE, Box::new(consumer));

    let ergebnis = dispatcher.dispatch(read_transactions_envelope());

    // Terminal → `Warning` über die **redigierte** `Display` des Domänen-Fehlers (nie der Rohwert).
    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason }) if !reason.contains(leck)),
        "erwartet redigiertes Dead-Letter: {ergebnis:?}"
    );
    // Nichts committet: die Arbeitseinheit fiel zurück.
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
