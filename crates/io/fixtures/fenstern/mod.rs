//! Die **test-lokalen** Fixtures/Helfer des `fenstern`-Integrationstests — Seam-Fakes, die
//! Fake-Arbeitseinheit, die Dispatcher-Verdrahtung, die Broker-Schleife und die Input-/
//! Expected-Builder, die der nackte Test in `tests/` per Modul-Include
//! (`#[path = …] mod fixtures;`) hereinzieht.
//!
//! **Der Test spielt den Broker:** [`fahre_lauf`] reicht die Anstoß-Envelope hinein und speist jede
//! **committete** Nachricht, die dieser Slice selbst konsumiert, wieder in den Dispatcher — genau
//! das macht die **Selbst-Fortsetzung** des Pagers sichtbar. Fremde Nachrichtentypen
//! (`WindowClosed`, `HistoryComplete`) gehen stromabwärts nach `auswerten` und bleiben in der
//! Outbox liegen.
//!
//! **Atomarität als Fake:** die Arbeitseinheit ([`FakeUnit`]) **ist** die gestagete Fenster-Ablage
//! (`AnalysisWindowStoreSeam`) **und** die Outbox (`OutboxPublisher`); die Handler des Slice werden
//! von `io` **aus ihr** gebaut, sodass Staging und Commit dieselbe Unit teilen. Sie hält die
//! gestagete Arbeit in eigenen `pending`-Puffern und flusht sie beim Commit in den geteilten
//! `committed`-Sink; wird sie ohne Commit fallengelassen (Handler-Fehler), verwirft das Fallenlassen
//! ihre `pending`-Arbeit (Rollback ohne Sonderfall). Das **Lesen** (`ObfuscatedWindowSeam`) liegt
//! **außerhalb** der Transaktion — der Aufrufer reicht es separat herein.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::rc::Rc;

use fenstern_adapter::{
    AnalysisWindowRow, AnalysisWindowStoreSeam, ObfuscatedRow, ObfuscatedWindowSeam,
    SourceUnavailable, StoreUnavailable,
};
use io::Dispatcher;
use io::fenstern::UnitFactory;
use io::outbox::{OutboxCommand, OutboxError, OutboxMessage, OutboxPublisher, UnitOfWork};

#[path = "../commit_failed.rs"]
mod commit_failed;
use commit_failed::CommitFailed;

/// Seitengröße des Tests: klein genug, dass der Bestand unten in **drei** Seiten zerfällt.
pub(crate) const WINDOW_DAYS: u32 = 3;

/// Obergrenze der Broker-Schleife: die **Terminierung** der Selbst-Fortsetzung ist Teil dessen, was
/// hier geprüft wird — ohne Schranke würde ein Regress den Test hängen lassen statt scheitern.
const MAX_RUNDEN: usize = 32;

// --- Lese-Seam-Fake (minimaler Stub über einem Tag→Zeile-Bestand) -----------------------

/// Fake der `obfuskiert`-Ablage: hält die arrangierten Zeilen je Buchungstag (Tag-Stamp). Die drei
/// Methoden spielen genau die drei Queries, die der echte io absetzt — Spannen-Lesen,
/// `MAX(booking_day)` und `MAX(booking_day) WHERE booking_day < ?`.
#[derive(Default)]
pub(crate) struct FakeSource {
    rows: Vec<(i64, ObfuscatedRow)>,
    fail: Option<&'static str>,
}

impl FakeSource {
    /// Arrangiert den Bestand als `(Buchungstag, Buchungs-ID)`-Paare, alle in einer Währung.
    pub(crate) fn mit(rows: &[(i64, &str)]) -> Self {
        Self {
            rows: rows
                .iter()
                .map(|(day, id)| (*day, zeile(id, "EUR")))
                .collect(),
            fail: None,
        }
    }

    /// Ersetzt die Zeile zu einem Tag — für den Datensatz mit leerem Pflichtfeld.
    pub(crate) fn mit_zeile_am(mut self, day: i64, row: ObfuscatedRow) -> Self {
        self.rows.retain(|(tag, _)| *tag != day);
        self.rows.push((day, row));
        self.rows.sort_by(|(a, _), (b, _)| b.cmp(a));
        self
    }

    fn tage(&self) -> impl Iterator<Item = i64> + '_ {
        self.rows.iter().map(|(day, _)| *day)
    }
}

impl ObfuscatedWindowSeam for FakeSource {
    fn read(&self, from: i64, to: i64) -> Result<Vec<ObfuscatedRow>, SourceUnavailable> {
        match self.fail {
            Some(message) => Err(SourceUnavailable {
                message: message.to_string(),
            }),
            None => Ok(self
                .rows
                .iter()
                .filter(|(day, _)| (from..=to).contains(day))
                .map(|(_, row)| row.clone())
                .collect()),
        }
    }

    fn latest(&self) -> Result<Option<i64>, SourceUnavailable> {
        Ok(self.tage().max())
    }

    fn latest_before(&self, day: i64) -> Result<Option<i64>, SourceUnavailable> {
        Ok(self.tage().filter(|tag| *tag < day).max())
    }
}

pub(crate) fn zeile(id: &str, currency: &str) -> ObfuscatedRow {
    ObfuscatedRow {
        id: id.to_string(),
        currency: currency.to_string(),
    }
}

// --- Fake-Arbeitseinheit: Ports = die Unit, die committet -------------------------------

/// Die geteilte, committete Wahrheit hinter allen Arbeitseinheiten.
#[derive(Clone, Default)]
pub(crate) struct FakeDb {
    committed: Rc<RefCell<Committed>>,
    /// Ab dem wievielten Commit dieser Lauf scheitert (`None` = nie) — so lässt sich ein Fehlschlag
    /// **mitten** in der Kette arrangieren, nicht nur am Anfang.
    fail_commit_ab: Option<usize>,
}

#[derive(Default)]
struct Committed {
    /// Je abgelegtem Fenster die von der Ablage **neu vergebene** ID und die Zeile.
    windows: Vec<(String, AnalysisWindowRow)>,
    messages: Vec<(String, String)>,
    commits: usize,
}

impl FakeDb {
    pub(crate) fn commit_scheitert_ab(runde: usize) -> Self {
        Self {
            fail_commit_ab: Some(runde),
            ..Self::default()
        }
    }

    pub(crate) fn committed_windows(&self) -> Vec<AnalysisWindowRow> {
        self.committed
            .borrow()
            .windows
            .iter()
            .map(|(_, window)| window.clone())
            .collect()
    }

    /// Die committeten Nachrichten als `(message_type, partition_key)` — die Outbox, wie ein Broker
    /// sie sähe.
    pub(crate) fn committed_messages(&self) -> Vec<(String, String)> {
        self.committed.borrow().messages.clone()
    }

    /// Die seit `ab` neu hinzugekommenen Nachrichten — der Nachschub der Broker-Schleife.
    fn messages_ab(&self, ab: usize) -> Vec<(String, String)> {
        self.committed
            .borrow()
            .messages
            .iter()
            .skip(ab)
            .cloned()
            .collect()
    }
}

impl UnitFactory for FakeDb {
    type Unit = FakeUnit;

    fn begin(&self, _ack: io::MessageId<'_>) -> FakeUnit {
        FakeUnit {
            committed: Rc::clone(&self.committed),
            pending: RefCell::new(Committed::default()),
            fail_commit_ab: self.fail_commit_ab,
        }
    }
}

/// Eine Arbeitseinheit: die gestagete Fenster-Ablage **und** die Outbox in einem. `save`/`publish`
/// stagen in eigene `pending`-Puffer; `commit` flusht sie in den geteilten `committed`-Sink (oder
/// scheitert, wenn dieser Commit arrangiert scheitern soll). Ohne Commit (Handler-Fehler) verwirft
/// das Fallenlassen der Unit die `pending`-Arbeit — Rollback ohne Drop-Impl.
pub(crate) struct FakeUnit {
    committed: Rc<RefCell<Committed>>,
    pending: RefCell<Committed>,
    fail_commit_ab: Option<usize>,
}

impl AnalysisWindowStoreSeam for FakeUnit {
    fn save(&self, window: &AnalysisWindowRow) -> Result<String, StoreUnavailable> {
        let mut pending = self.pending.borrow_mut();
        // Deterministische, je abgelegtem Fenster eindeutige ID (der Fake spielt `INSERT … RETURNING id`).
        let id = format!(
            "fenster-{}",
            self.committed.borrow().windows.len() + pending.windows.len() + 1
        );
        pending.windows.push((id.clone(), window.clone()));
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
        let runde = self.committed.borrow().commits;
        if self.fail_commit_ab.is_some_and(|ab| runde + 1 >= ab) {
            return Err(CommitFailed);
        }
        let pending = self.pending.into_inner();
        let mut committed = self.committed.borrow_mut();
        committed.windows.extend(pending.windows);
        committed.messages.extend(pending.messages);
        committed.commits += 1;
        Ok(())
    }
}

// --- Verdrahtung + Broker-Schleife ------------------------------------------------------

/// Registriert **beide** Kommandotypen des Slice beim einen Dispatcher — der Anstoß und der
/// Fenster-Schritt werden uniform über ihren `message_type` aufgelöst.
pub(crate) fn dispatcher_mit(source: &FakeSource, db: FakeDb) -> Dispatcher<'_> {
    let seitengröße = NonZeroU32::new(WINDOW_DAYS).expect("Seitengröße ist nicht null");
    let mut dispatcher = Dispatcher::new();
    assert!(
        dispatcher
            .register(
                io::fenstern::START_MESSAGE_TYPE,
                Box::new(io::fenstern::start_consumer(source, db.clone())),
            )
            .is_none(),
        "frische Registrierung darf keinen Consumer verdrängen"
    );
    assert!(
        dispatcher
            .register(
                io::fenstern::CONSUMED_MESSAGE_TYPE,
                Box::new(io::fenstern::consumer(seitengröße, source, db)),
            )
            .is_none(),
        "frische Registrierung darf keinen Consumer verdrängen"
    );
    dispatcher
}

pub(crate) fn envelope(message_type: &str, partition_key: &str) -> OutboxMessage {
    OutboxMessage {
        // Die Envelope-`id` ist eine io-vergebene Message-UUID — bewusst **verschieden** vom
        // Anker im `partition_key`, damit der Test den Handshake über `partition_key` erzwingt.
        id: format!("msg-uuid-{message_type}-{partition_key}"),
        message_type: message_type.to_string(),
        partition_key: partition_key.to_string(),
        content: String::new(),
        occurred_on_utc: 0,
        attempt: 0,
    }
}

/// Was **dieser** Slice konsumiert — alles andere (`WindowClosed`/`HistoryComplete`) reist
/// stromabwärts und wird von der Schleife nicht zurückgespeist.
fn wird_konsumiert(message_type: &str) -> bool {
    message_type == io::fenstern::START_MESSAGE_TYPE
        || message_type == io::fenstern::CONSUMED_MESSAGE_TYPE
}

/// Fährt den **ganzen Lauf**: Anstoß hinein, dann jede committete, selbst-konsumierte Nachricht
/// wieder hinein — bis nichts mehr nachwächst. Bricht beim ersten Fehler ab (der Broker würde die
/// Nachricht dead-lettern bzw. retryen, statt die Kette fortzusetzen).
///
/// Die Schleife ist **bewusst gedeckelt**: terminierte die Selbst-Fortsetzung nicht, hinge der Test
/// nicht endlos, sondern bräche nach [`MAX_RUNDEN`] ab. Der Deckel **meldet sich dabei laut** — sonst
/// liefe ein Nicht-Terminieren als `Ok(())` durch und fiele nur denjenigen Tests auf, die die
/// **ganze** Outbox-Kette vergleichen; ein künftiger Test, der das nicht tut, würde den Regress
/// stillschweigend bestehen.
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test-Fixture: ein nicht terminierender Pager ist ein Regress der Selbst-Fortsetzung \
              und muss den Test hart scheitern lassen, nicht als Err durch die Assertions reisen — \
              der Rückgabewert gehört allein der Disposition der Nachrichtenverarbeitung"
)]
pub(crate) fn fahre_lauf(dispatcher: &Dispatcher<'_>, db: &FakeDb) -> Result<(), OutboxError> {
    let mut offen = VecDeque::from([envelope(io::fenstern::START_MESSAGE_TYPE, "")]);
    let mut gelesen = 0;
    for _ in 0..MAX_RUNDEN {
        let Some(nachricht) = offen.pop_front() else {
            break;
        };
        dispatcher.dispatch(nachricht)?;
        let nachschub = db.messages_ab(gelesen);
        gelesen += nachschub.len();
        offen.extend(
            nachschub
                .iter()
                .filter(|(typ, _)| wird_konsumiert(typ))
                .map(|(typ, key)| envelope(typ, key)),
        );
    }
    assert!(
        offen.is_empty(),
        "Broker-Schleife erreichte MAX_RUNDEN ({MAX_RUNDEN}) ohne Terminierung; noch {} Nachricht(en) offen",
        offen.len()
    );
    Ok(())
}

// --- Fixtures ---------------------------------------------------------------------------

/// Ein Bestand, der bei Seitengröße 3 in genau **drei** Seiten zerfällt:
/// `[98…100]` → b-1/b-2, `[93…95]` → b-3, `[88…90]` → b-4. Die Lücken (97…96, 94…91) werden
/// übersprungen, weil jeder Folge-Anker ein **echter** Buchungstag ist.
pub(crate) fn bestand() -> FakeSource {
    FakeSource::mit(&[(100, "b-1"), (99, "b-2"), (95, "b-3"), (90, "b-4")])
}

pub(crate) fn fenster(from: i64, to: i64, booking_ids: &[&str]) -> AnalysisWindowRow {
    AnalysisWindowRow {
        from,
        to,
        currency: "EUR".to_string(),
        booking_ids: booking_ids.iter().map(|id| id.to_string()).collect(),
    }
}

pub(crate) fn nachricht(message_type: &str, partition_key: &str) -> (String, String) {
    (message_type.to_string(), partition_key.to_string())
}
