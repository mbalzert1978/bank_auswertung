//! io-Integrationstest ([Issue 0020](../../../docs/issues/0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md)):
//! ein `ReadTransactions`-Envelope läuft **end-to-end** durch die generische Consumer-Maschinerie
//! in `io` → den echten `einlesen`-Slice → bis zur publizierten „Ingested"-Nachricht. Getestet
//! wird die **io-Verdrahtung** (Envelope-Extraktion, `CommandDecoder`-Aufruf, atomarer
//! `UnitOfWork`-Commit, message_type-Auflösung, Dead-Letter-Taktik), nicht das Slice-Verhalten
//! (das prüft der `einlesen`-Spec). Alle Seams sind Fakes; der echte SQLite-/`sqlx`-Teil bleibt
//! draußen (Build-Order Schritt 5).
//!
//! **Atomarität als Fake:** die Arbeitseinheit (`FakeUnit`) **ist** der gestagete Speicher
//! (`StoreSeam`) **und** die Outbox (`OutboxPublisher`); der `einlesen`-Handler wird von `io` **aus
//! ihr** gebaut, sodass Staging und Commit dieselbe Unit teilen. Sie hält die gestagete Arbeit in
//! eigenen `pending`-Puffern und flusht sie beim Commit in den geteilten `committed`-Sink; wird sie
//! ohne Commit fallengelassen (Handler-Fehler), verwirft das Fallenlassen der Unit ihre
//! `pending`-Arbeit (Rollback ohne Sonderfall) — eine gestagete Schreibung committet also mit der
//! Nachricht oder gar nicht.
//!
//! Die Seam-Fakes (`FakeSource`/`FakeCalendar`/`FakeMoney`) sind **bewusst minimale Stubs** für die
//! zwei Fixtures — kein Nachbau des echten Dekodierens (das prüft der `einlesen`-Spec hinter den
//! Seams). So gibt es keine „reiche" Parser-Duplikation, die gegen den Adapter driften könnte.

use std::cell::{Cell, RefCell};
use std::convert::Infallible;
use std::rc::Rc;

use einlesen_adapter::{
    CalendarDay, CalendarSeam, DecodedAmount, Line, MoneySeam, RawRecord, Sign, SourceSeam,
    SourceUnavailable, StoreSeam, StoreUnavailable,
};
use io::Dispatcher;
use io::einlesen::UnitFactory;
use shared_kernel::{OutboxCommand, OutboxError, OutboxMessage, OutboxPublisher, UnitOfWork};

// --- Seam-Fakes (minimale Stubs für die Fixtures) ---------------------------------------

/// Fake-Quelle: liefert die arrangierten Roh-Zeilen (die Beschaffung gelingt).
struct FakeSource {
    records: Vec<RawRecord>,
}

impl SourceSeam for FakeSource {
    fn fetch(&self) -> Result<Vec<RawRecord>, SourceUnavailable> {
        Ok(self.records.clone())
    }
}

/// Minimaler Kalender-Stub: zerlegt „TT.MM.JJJJ" ohne Bereichsprüfung (die Fixtures sind gültig;
/// ungültige Daten prüft der `einlesen`-Spec, nicht dieser Maschinen-Test).
struct FakeCalendar;

impl CalendarSeam for FakeCalendar {
    fn parse(&self, raw: &str) -> Option<CalendarDay> {
        let mut parts = raw.split('.');
        Some(CalendarDay {
            day: parts.next()?.parse().ok()?,
            month: parts.next()?.parse().ok()?,
            year: parts.next()?.parse().ok()?,
        })
    }
}

/// Minimaler Geld-Stub: das negative Fixture-Format dekodiert zu Cent, alles Nicht-Numerische
/// (z. B. eine IBAN im Betragsfeld) zu `None` — genug, um „gültig vs. terminal" zu unterscheiden.
struct FakeMoney;

impl MoneySeam for FakeMoney {
    fn decode(&self, raw: &str) -> Option<DecodedAmount> {
        let digits = raw.strip_prefix('-')?;
        let cent = digits.replace([',', '.'], "").parse().ok()?;
        Some(DecodedAmount {
            cent,
            sign: Sign::Negative,
        })
    }
}

// --- Fake-Arbeitseinheit: Ports = die Unit, die committet -------------------------------

/// Die geteilte, committete Wahrheit hinter allen Arbeitseinheiten.
#[derive(Clone, Default)]
struct FakeDb {
    committed: Rc<RefCell<Committed>>,
}

#[derive(Default)]
struct Committed {
    lines: Vec<(String, Line)>,
    messages: Vec<(String, String)>,
}

impl FakeDb {
    fn committed_lines(&self) -> Vec<Line> {
        self.committed
            .borrow()
            .lines
            .iter()
            .map(|(_, line)| line.clone())
            .collect()
    }

    fn committed_line_ids(&self) -> Vec<String> {
        self.committed
            .borrow()
            .lines
            .iter()
            .map(|(id, _)| id.clone())
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
            next_id: Cell::new(0),
        }
    }
}

/// Eine Arbeitseinheit: der gestagete Speicher **und** die Outbox in einem. `store`/`publish` stagen
/// in eigene `pending`-Puffer; `commit` flusht sie in den geteilten `committed`-Sink. Ohne Commit
/// (Handler-Fehler) verwirft das Fallenlassen der Unit die `pending`-Arbeit — Rollback ohne Drop-Impl.
struct FakeUnit {
    committed: Rc<RefCell<Committed>>,
    pending: RefCell<Committed>,
    next_id: Cell<u32>,
}

impl StoreSeam for FakeUnit {
    fn store(&self, line: &Line) -> Result<String, StoreUnavailable> {
        let id = self.next_id.get() + 1;
        self.next_id.set(id);
        let id = format!("ingested-{id}");
        self.pending
            .borrow_mut()
            .lines
            .push((id.clone(), line.clone()));
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
    type Error = Infallible;

    fn commit(self) -> Result<(), Infallible> {
        let pending = self.pending.into_inner();
        let mut committed = self.committed.borrow_mut();
        committed.lines.extend(pending.lines);
        committed.messages.extend(pending.messages);
        Ok(())
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
    let consumer = io::einlesen::consumer(&source, &FakeCalendar, &FakeMoney, db.clone());

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
    let consumer = io::einlesen::consumer(&source, &FakeCalendar, &FakeMoney, db.clone());

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
