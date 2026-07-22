//! Die **test-lokalen** Fixtures/Helfer des `einlesen`-Integrationstests — Seam-Fakes, die
//! Fake-Arbeitseinheit und die Input-Builder, die der nackte Test in `tests/` per Modul-Include
//! (`#[path = …] mod fixtures;`) hereinzieht.
//!
//! **Atomarität als Fake:** die Arbeitseinheit ([`FakeUnit`]) **ist** der gestagete Speicher
//! (`StoreSeam`) **und** die Outbox (`OutboxPublisher`); der `einlesen`-Handler wird von `io` **aus
//! ihr** gebaut, sodass Staging und Commit dieselbe Unit teilen. Sie hält die gestagete Arbeit in
//! eigenen `pending`-Puffern und flusht sie beim Commit in den geteilten `committed`-Sink; wird sie
//! ohne Commit fallengelassen (Handler-Fehler), verwirft das Fallenlassen der Unit ihre
//! `pending`-Arbeit (Rollback ohne Sonderfall) — eine gestagete Schreibung committet also mit der
//! Nachricht oder gar nicht.
//!
//! **Geld/Kalender/Quelle sind keine Fakes mehr:** seit
//! [Issue 0050](../../../../docs/issues/0050-money-calendar-decode-seams-real.md) steckt der Test die
//! echten io-Adapter (`io::einlesen::GermanMoney`/`GermanCalendar`) ein, und seit
//! [Issue 0051](../../../../docs/issues/0051-einlesen-csv-camt-v8-quelle-real-hinter-source-seam.md)
//! auch den echten CSV-CAMT-v8-Reader ([`csv_source`], der eine synthetische Fixture-Datei über
//! `fs` + `csv` liest). Fake bleibt nur die Arbeitseinheit ([`FakeUnit`]) — der echte
//! SQLite-/`rusqlite`-Teil folgt in Build-Order Schritt 5.

use std::cell::{Cell, RefCell};
use std::convert::Infallible;
use std::rc::Rc;

use einlesen_adapter::{CalendarDay, Currency, Direction, Line, StoreSeam, StoreUnavailable};
use io::einlesen::{CsvCamtV8Source, UnitFactory};
use io::outbox::{OutboxCommand, OutboxMessage, OutboxPublisher, UnitOfWork};

// --- Echte Quelle: der CSV-CAMT-v8-Reader auf einer synthetischen Fixture-Datei ----------

/// Baut den echten [`CsvCamtV8Source`] auf die benannte Fixture-Datei unter
/// `fixtures/einlesen/csv/` (Pfad relativ zum io-Crate). So läuft der Beschaffungs-Seam im Test
/// gegen den **echten** Reader (`fs` + `csv`), nicht mehr gegen einen Fake.
pub(crate) fn csv_source(name: &str) -> CsvCamtV8Source {
    let path = format!(
        "{}/fixtures/einlesen/csv/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    CsvCamtV8Source::new(path)
}

/// Die erwartete `Line`-Projektion der einzigen Zeile aus `gueltig.csv` (EDEKA, `-50,00 EUR`) —
/// unabhängige Wahrheit für den Feld-Treue-Vergleich der committeten Zeile.
pub(crate) fn erwartete_gueltige_zeile() -> Line {
    Line {
        ordering_account: "DE89370400440532013000".to_string(),
        counter_account: Some("DE11520513735120710131".to_string()),
        amount_cent: 5000,
        currency: Currency::Euro,
        direction: Direction::Outgoing,
        recipient_name: Some("EDEKA SAGT DANKE".to_string()),
        creditor_id: Some("DE98ZZZ09999999999".to_string()),
        booking_day: CalendarDay {
            year: 2024,
            month: 6,
            day: 13,
        },
        value_date: CalendarDay {
            year: 2024,
            month: 6,
            day: 14,
        },
        purpose: Some("Einkauf".to_string()),
        booking_text: Some("LASTSCHRIFT".to_string()),
    }
}

// --- Fake-Arbeitseinheit: Ports = die Unit, die committet -------------------------------

/// Die geteilte, committete Wahrheit hinter allen Arbeitseinheiten.
#[derive(Clone, Default)]
pub(crate) struct FakeDb {
    committed: Rc<RefCell<Committed>>,
}

#[derive(Default)]
struct Committed {
    lines: Vec<(String, Line)>,
    messages: Vec<(String, String)>,
}

impl FakeDb {
    pub(crate) fn committed_lines(&self) -> Vec<Line> {
        self.committed
            .borrow()
            .lines
            .iter()
            .map(|(_, line)| line.clone())
            .collect()
    }

    pub(crate) fn committed_line_ids(&self) -> Vec<String> {
        self.committed
            .borrow()
            .lines
            .iter()
            .map(|(id, _)| id.clone())
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
            next_id: Cell::new(0),
        }
    }
}

/// Eine Arbeitseinheit: der gestagete Speicher **und** die Outbox in einem. `store`/`publish` stagen
/// in eigene `pending`-Puffer; `commit` flusht sie in den geteilten `committed`-Sink. Ohne Commit
/// (Handler-Fehler) verwirft das Fallenlassen der Unit die `pending`-Arbeit — Rollback ohne Drop-Impl.
pub(crate) struct FakeUnit {
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
