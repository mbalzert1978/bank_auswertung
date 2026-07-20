//! io-Integrationstest ([Issue 0039](../../../docs/issues/0039-io-fenstern-end-to-end-ueber-generische-outboxconsumer.md)):
//! ein `StartAnalysis` läuft **end-to-end** durch die generische Consumer-Maschinerie in `io` → den
//! echten `fenstern`-Slice → über mehrere Seiten bis zum abschließenden `HistoryComplete`. Getestet
//! wird die **io-Verdrahtung** (Envelope-Extraktion, `CommandDecoder`-Aufruf, atomarer
//! `UnitOfWork`-Commit, uniforme message_type-Auflösung **beider** Kommandotypen, Fehler-Taktik),
//! nicht das Slice-Verhalten (das prüft der `fenstern`-Spec). Alle Seams sind Fakes; der echte
//! SQLite-/`sqlx`-Teil bleibt draußen (Build-Order Schritt 5).
//!
//! **Der Test spielt den Broker:** er reicht die Anstoß-Envelope hinein und speist jede
//! **committete** Nachricht, die dieser Slice selbst konsumiert, wieder in den Dispatcher — genau
//! das macht die **Selbst-Fortsetzung** des Pagers sichtbar. Fremde Nachrichtentypen
//! (`WindowClosed`, `HistoryComplete`) gehen stromabwärts nach `auswerten` und bleiben in der
//! Outbox liegen.
//!
//! **Atomarität als Fake:** die Arbeitseinheit (`FakeUnit`) **ist** die gestagete Fenster-Ablage
//! (`AnalysisWindowStoreSeam`) **und** die Outbox (`OutboxPublisher`); die Handler des Slice werden
//! von `io` **aus ihr** gebaut, sodass Staging und Commit dieselbe Unit teilen. Sie hält die
//! gestagete Arbeit in eigenen `pending`-Puffern und flusht sie beim Commit in den geteilten
//! `committed`-Sink; wird sie ohne Commit fallengelassen (Handler-Fehler), verwirft das Fallenlassen
//! ihre `pending`-Arbeit (Rollback ohne Sonderfall). Das **Lesen** (`ObfuscatedWindowSeam`) liegt
//! **außerhalb** der Transaktion — der Aufrufer reicht es separat herein.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::num::NonZeroU32;
use std::rc::Rc;

use fenstern_adapter::{
    AnalysisWindowRow, AnalysisWindowStoreSeam, ObfuscatedRow, ObfuscatedWindowSeam,
    SourceUnavailable, StoreUnavailable,
};
use io::Dispatcher;
use io::fenstern::UnitFactory;
use io::outbox::{OutboxCommand, OutboxError, OutboxMessage, OutboxPublisher, UnitOfWork};

/// Seitengröße des Tests: klein genug, dass der Bestand unten in **drei** Seiten zerfällt.
const WINDOW_DAYS: u32 = 3;

/// Obergrenze der Broker-Schleife: die **Terminierung** der Selbst-Fortsetzung ist Teil dessen, was
/// hier geprüft wird — ohne Schranke würde ein Regress den Test hängen lassen statt scheitern.
const MAX_RUNDEN: usize = 32;

// --- Lese-Seam-Fake (minimaler Stub über einem Tag→Zeile-Bestand) -----------------------

/// Fake der `obfuskiert`-Ablage: hält die arrangierten Zeilen je Buchungstag (Tag-Stamp). Die drei
/// Methoden spielen genau die drei Queries, die der echte io absetzt — Spannen-Lesen,
/// `MAX(booking_day)` und `MAX(booking_day) WHERE booking_day < ?`.
#[derive(Default)]
struct FakeSource {
    rows: Vec<(i64, ObfuscatedRow)>,
    fail: Option<&'static str>,
}

impl FakeSource {
    /// Arrangiert den Bestand als `(Buchungstag, Buchungs-ID)`-Paare, alle in einer Währung.
    fn mit(rows: &[(i64, &str)]) -> Self {
        Self {
            rows: rows
                .iter()
                .map(|(day, id)| (*day, zeile(id, "EUR")))
                .collect(),
            fail: None,
        }
    }

    /// Ersetzt die Zeile zu einem Tag — für den Datensatz mit leerem Pflichtfeld.
    fn mit_zeile_am(mut self, day: i64, row: ObfuscatedRow) -> Self {
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

fn zeile(id: &str, currency: &str) -> ObfuscatedRow {
    ObfuscatedRow {
        id: id.to_string(),
        currency: currency.to_string(),
    }
}

// --- Fake-Arbeitseinheit: Ports = die Unit, die committet -------------------------------

/// Ein Commit ist gescheitert (io-interner Retry-Ursprung). Trägt eine **Transaktions**-Diagnose,
/// nie einen Zeilenwert (Redaktions-Pflicht des Commit-Pfades).
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
    fn commit_scheitert_ab(runde: usize) -> Self {
        Self {
            fail_commit_ab: Some(runde),
            ..Self::default()
        }
    }

    fn committed_windows(&self) -> Vec<AnalysisWindowRow> {
        self.committed
            .borrow()
            .windows
            .iter()
            .map(|(_, window)| window.clone())
            .collect()
    }

    /// Die committeten Nachrichten als `(message_type, partition_key)` — die Outbox, wie ein Broker
    /// sie sähe.
    fn committed_messages(&self) -> Vec<(String, String)> {
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

    fn begin(&self) -> FakeUnit {
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
struct FakeUnit {
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
fn dispatcher_mit(source: &FakeSource, db: FakeDb) -> Dispatcher<'_> {
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

fn envelope(message_type: &str, partition_key: &str) -> OutboxMessage {
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
/// nicht endlos, sondern bräche nach [`MAX_RUNDEN`] ab — woraufhin die Nachrichten-Assertion des
/// Aufrufers scheitert, weil dann Nachschub übrig geblieben ist. Der Deckel braucht darum keine
/// eigene Zusicherung; die Terminierung prüft der Test dort, wo er die ganze Kette vergleicht.
fn fahre_lauf(dispatcher: &Dispatcher<'_>, db: &FakeDb) -> Result<(), OutboxError> {
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
    Ok(())
}

// --- Fixtures ---------------------------------------------------------------------------

/// Ein Bestand, der bei Seitengröße 3 in genau **drei** Seiten zerfällt:
/// `[98…100]` → b-1/b-2, `[93…95]` → b-3, `[88…90]` → b-4. Die Lücken (97…96, 94…91) werden
/// übersprungen, weil jeder Folge-Anker ein **echter** Buchungstag ist.
fn bestand() -> FakeSource {
    FakeSource::mit(&[(100, "b-1"), (99, "b-2"), (95, "b-3"), (90, "b-4")])
}

fn fenster(from: i64, to: i64, booking_ids: &[&str]) -> AnalysisWindowRow {
    AnalysisWindowRow {
        from,
        to,
        currency: "EUR".to_string(),
        booking_ids: booking_ids.iter().map(|id| id.to_string()).collect(),
    }
}

fn nachricht(message_type: &str, partition_key: &str) -> (String, String) {
    (message_type.to_string(), partition_key.to_string())
}

// --- Happy Path: StartAnalysis → drei Seiten → HistorieVollständig ----------------------

#[test]
fn ein_start_analysis_laeuft_ueber_mehrere_seiten_bis_historie_vollstaendig() {
    let source = bestand();
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, db.clone());

    let ergebnis = fahre_lauf(&dispatcher, &db);

    assert_eq!(ergebnis, Ok(()));
    // Je Seite **ein** abgelegtes Auswertungsfenster, neu→alt, jeweils nur mit den Fremdschlüsseln
    // der Zeilen, die auf der Seite liegen.
    assert_eq!(
        db.committed_windows(),
        vec![
            fenster(98, 100, &["b-1", "b-2"]),
            fenster(93, 95, &["b-3"]),
            fenster(88, 90, &["b-4"]),
        ]
    );
    // Die ganze Kette in der Outbox: der Anstoß eröffnet, jede Seite schließt ihr Fenster und reiht
    // die nächste ein, und **genau ein** `HistoryComplete` mit der Lauf-Spanne schließt ab. Der
    // Lauf-Gipfel (100) reist unverändert durch alle Anker.
    assert_eq!(
        db.committed_messages(),
        vec![
            nachricht("AdvanceWindow", "100:100"),
            nachricht("WindowClosed", "fenster-1"),
            nachricht("AdvanceWindow", "95:100"),
            nachricht("WindowClosed", "fenster-2"),
            nachricht("AdvanceWindow", "90:100"),
            nachricht("WindowClosed", "fenster-3"),
            nachricht("HistoryComplete", "88:100"),
        ]
    );
}

// --- Beide Kommandotypen: uniforme Auflösung über den einen Dispatcher -------------------

#[test]
fn beide_kommandotypen_werden_vom_selben_dispatcher_aufgeloest() {
    let source = bestand();
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, db.clone());

    // Der Anstoß: aufgelöst, verarbeitet — er publiziert nur das erste Fenster-Kommando und legt
    // selbst **nichts** ab (einzweckig).
    assert_eq!(
        dispatcher.dispatch(envelope(io::fenstern::START_MESSAGE_TYPE, "")),
        Ok(())
    );
    assert!(db.committed_windows().is_empty());
    assert_eq!(
        db.committed_messages(),
        vec![nachricht("AdvanceWindow", "100:100")]
    );

    // Der Fenster-Schritt: derselbe Dispatcher, anderer `message_type`, anderer Handler — er
    // materialisiert das Fenster.
    assert_eq!(
        dispatcher.dispatch(envelope(io::fenstern::CONSUMED_MESSAGE_TYPE, "100:100")),
        Ok(())
    );
    assert_eq!(
        db.committed_windows(),
        vec![fenster(98, 100, &["b-1", "b-2"])]
    );
}

// --- Leerer Bestand: erfolgreich, ohne jede Nachricht ------------------------------------

#[test]
fn ein_leerer_bestand_endet_erfolgreich_ohne_nachricht() {
    let source = FakeSource::default();
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, db.clone());

    let ergebnis = fahre_lauf(&dispatcher, &db);

    // Erschöpfung schon bei Schritt 0: kein Fehler, aber auch kein `HistoryComplete` mit entarteter
    // Spanne — `auswerten` wird gar nicht erst angestoßen.
    assert_eq!(ergebnis, Ok(()));
    assert!(db.committed_windows().is_empty());
    assert!(db.committed_messages().is_empty());
}

// --- Terminaler Handler-Fehler: redigiert, nichts committet, Kette bricht ab -------------

#[test]
fn ein_terminaler_fehler_rollt_die_ganze_arbeitseinheit_zurueck() {
    // Pflichtfeld leer (Währung) auf der **zweiten** Seite → `MissingMandatoryField` (terminal).
    let source = bestand().mit_zeile_am(95, zeile("b-3", ""));
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, db.clone());

    let ergebnis = fahre_lauf(&dispatcher, &db);

    // Der Grund trägt nur den statischen Feldnamen, nie eine Buchungs-ID.
    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason })
            if reason.contains("currency") && !reason.contains("b-3")),
        "erwartet redigiertes terminales Dead-Letter: {ergebnis:?}"
    );
    // Die erste Seite steht (sie committete sauber); von der zweiten ist **nichts** da — weder das
    // Fenster noch der `WindowClosed`-Zeiger noch eine Fortsetzung. Keine halb-fortgesetzte Kette.
    assert_eq!(
        db.committed_windows(),
        vec![fenster(98, 100, &["b-1", "b-2"])]
    );
    assert_eq!(
        db.committed_messages(),
        vec![
            nachricht("AdvanceWindow", "100:100"),
            nachricht("WindowClosed", "fenster-1"),
            nachricht("AdvanceWindow", "95:100"),
        ]
    );
}

// --- Commit-Fehler: der einzige Retry-Fall, nichts committet -----------------------------

#[test]
fn ein_commit_fehler_hinterlaesst_keine_halb_fortgesetzte_kette() {
    // Der **zweite** Commit scheitert: der Anstoß committet noch, der erste Fenster-Schritt nicht.
    let source = bestand();
    let db = FakeDb::commit_scheitert_ab(2);
    let dispatcher = dispatcher_mit(&source, db.clone());

    let ergebnis = fahre_lauf(&dispatcher, &db);

    // Der Handler gelingt, aber der Commit scheitert → `Retry` (nicht `Warning`).
    assert!(
        matches!(&ergebnis, Err(OutboxError::Retry { reason }) if reason.contains("commit")),
        "ein Commit-Fehler ist der einzige io-interne Retry-Ursprung: {ergebnis:?}"
    );
    // Ablage **und** beide Publishes des gescheiterten Schrittes fielen gemeinsam zurück: nur die
    // Nachricht des Anstoßes steht, kein Fenster.
    assert!(db.committed_windows().is_empty());
    assert_eq!(
        db.committed_messages(),
        vec![nachricht("AdvanceWindow", "100:100")]
    );
}

// --- Blanker Anker: Decoder `None` → Critical auf Envelope-Ebene -------------------------

#[test]
fn ein_ankerloser_partition_key_wird_critical() {
    let source = bestand();
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, db.clone());

    // Ein `partition_key` ohne die Form `"{anchor}:{run_top}"` ist kein gültiger Anker → Decoder
    // `None` → `Critical` (Envelope-Ebene), nicht als Handler-Fehler durchgereicht.
    let ergebnis = dispatcher.dispatch(envelope(io::fenstern::CONSUMED_MESSAGE_TYPE, "   "));

    assert!(
        matches!(&ergebnis, Err(OutboxError::Critical { source, .. })
            if source == io::fenstern::CONSUMED_MESSAGE_TYPE),
        "ein ankerloser Zeiger muss zu Critical auf Envelope-Ebene führen: {ergebnis:?}"
    );
    assert!(db.committed_windows().is_empty());
}
