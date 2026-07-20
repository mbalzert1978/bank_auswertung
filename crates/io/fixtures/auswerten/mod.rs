//! Die **test-lokalen** Fixtures/Helfer des `auswerten`-Integrationstests — Seam-Fakes, die
//! Fake-Arbeitseinheit, die Dispatcher-Verdrahtung, die Broker-Schleife und die Input-/
//! Expected-Builder, die der nackte Test in `tests/` per Modul-Include
//! (`#[path = …] mod fixtures;`) hereinzieht.
//!
//! **Das Besondere gegenüber den Fixtures der Einzel-Slices:** hier stehen **zwei** Slices
//! hintereinander. Der Dispatcher trägt **vier** Consumer — `fenstern`s Anstoß und Fenster-Schritt,
//! `auswerten`s beide Reichweiten — und die Broker-Schleife speist **jede** committete Nachricht
//! zurück, nicht nur die selbst-konsumierten. Genau daran wird sichtbar, dass die beiden Slices
//! allein über die Outbox koppeln: `fenstern`s `WindowClosed`/`HistoryComplete` sind für es selbst
//! stromabwärts und für `auswerten` der Eingang, und **keiner der beiden Adapter weiß davon**
//! (ADR-0011).
//!
//! **Die Ablage spielt den Join** (den der echte io als SQL-`JOIN` fährt): `fenstern` legt eine
//! Fenster-Zeile ab, die auf die Buchungen nur per **Fremdschlüssel** zeigt (ADR-0012); `auswerten`
//! liest ein Fenster **zu einer ID** und erwartet die Buchungen **darin**. Der [`FakeJoin`] setzt
//! beides zusammen — aus den committeten Fenster-Zeilen und dem `obfuskiert`-Bestand. Dass diese
//! Naht existiert, ist Beschaffungs-Mechanik und liegt darum hier, nicht in einem der Slices.
//!
//! **Atomarität als Fake:** die Arbeitseinheit ([`FakeUnit`]) **ist** beide gestageten Ablagen
//! (`fenstern`s Fenster-Tabelle **und** `auswerten`s Ergebnis-Tabelle) **und** die Outbox; sie
//! flusht ihre `pending`-Puffer beim Commit in den geteilten `committed`-Sink. Wird sie ohne Commit
//! fallengelassen (Handler-Fehler), verwirft das Fallenlassen ihre `pending`-Arbeit — Rollback ohne
//! Sonderfall. Die **Lese**-Seams liegen **außerhalb** der Transaktion.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::rc::Rc;

use auswerten_adapter::{
    AnalysisWindowRow, AnalysisWindowSeam, BookingDirection, BookingRecipient, CategorizerSeam,
    CategorizerUnavailable, CategoryAnswerRow, CategoryRegistrySeam, CategoryRequestRow,
    CategoryRow, CounterAccountRow, EvaluationStoreSeam, MonthlyCashflowRow, OccurrenceRow,
    OccurrenceSeam, OwnAccountRow, OwnAccountsConfig, RegistryUnavailable, WindowBookingRow,
    WindowEvaluationRow,
};
use fenstern_adapter::{ObfuscatedRow, ObfuscatedWindowSeam};
use io::Dispatcher;
use io::auswerten::OccurrenceSink;
use io::outbox::{OutboxCommand, OutboxError, OutboxMessage, OutboxPublisher, UnitOfWork};

#[path = "../commit_failed.rs"]
mod commit_failed;
use commit_failed::CommitFailed;

/// Seitengröße des Laufs: groß genug, dass jedes Fenster mehrere Monats-Buckets überdeckt (Fenster
/// ≠ Zeitraum, CONTEXT.md), klein genug, dass der Bestand unten in **zwei** Fenster zerfällt.
pub(crate) const WINDOW_DAYS: u32 = 30;

/// Der Seed-Marker, unter dem die Token dieses Laufs entstanden sind — **nicht** der Seed
/// (ADR-0010). Fenster und Eigene-Konten-Config müssen denselben tragen, sonst verglichen sie Token
/// aus zwei verschiedenen Tokenisierungen.
pub(crate) const SEED_MARKER: &str = "marker-a";

/// Die Gläubiger-ID der wiederkehrenden Buchungsart des Laufs — der öffentliche Anker, unter dem
/// ihre Vorkommen über **beide** Fenster hinweg zusammenfinden.
pub(crate) const HAENDLER: &str = "DE98ZZZ09999999999";

/// Tag-Stamps (Tage seit Epoche, ADR-0013 Entscheidung C) — dieselben, an denen die
/// `auswerten`-Specs arbeiten.
pub(crate) const JAN_15: i64 = 19_737;
pub(crate) const JAN_31: i64 = 19_753;
pub(crate) const FEB_29: i64 = 19_782;

/// Obergrenze der Broker-Schleife: die **Terminierung** der Kette ist Teil dessen, was hier geprüft
/// wird — ohne Schranke würde ein Regress den Test hängen lassen statt scheitern.
const MAX_RUNDEN: usize = 32;

// --- Der `obfuskiert`-Bestand: die eine Quelle beider Lese-Seams ------------------------

/// Eine Buchung, wie `obfuskieren` sie abgelegt hat — **tokenisiert und gescrubbt**, stromabwärts
/// der Datenschutz-Grenze. Sie trägt, was die beiden Reichweiten verwerten; die beiden Lese-Seams
/// projizieren je ihren Ausschnitt daraus.
#[derive(Clone)]
pub(crate) struct Buchung {
    id: &'static str,
    day: i64,
    row: WindowBookingRow,
}

/// Der Bestand der `obfuskiert`-Ablage: die eine Wahrheit, aus der **beide** Slices lesen —
/// `fenstern` nur Tag + Währung (mehr braucht der Pager nicht), `auswerten` die ganze Zeile.
#[derive(Default)]
pub(crate) struct FakeBestand {
    bookings: Vec<Buchung>,
}

impl FakeBestand {
    pub(crate) fn mit(bookings: &[Buchung]) -> Self {
        Self {
            bookings: bookings.to_vec(),
        }
    }

    fn zeile(&self, id: &str) -> Option<&WindowBookingRow> {
        self.bookings
            .iter()
            .find(|booking| booking.id == id)
            .map(|booking| &booking.row)
    }

    fn tage(&self) -> impl Iterator<Item = i64> + '_ {
        self.bookings.iter().map(|booking| booking.day)
    }
}

/// `fenstern`s Lese-Seam über demselben Bestand: der Pager sieht **nur** ID und Währung — er
/// paginiert, er wertet nicht aus.
impl ObfuscatedWindowSeam for FakeBestand {
    fn read(
        &self,
        from: i64,
        to: i64,
    ) -> Result<Vec<ObfuscatedRow>, fenstern_adapter::SourceUnavailable> {
        Ok(self
            .bookings
            .iter()
            .filter(|booking| (from..=to).contains(&booking.day))
            .map(|booking| ObfuscatedRow {
                id: booking.id.to_string(),
                currency: "EUR".to_string(),
            })
            .collect())
    }

    fn latest(&self) -> Result<Option<i64>, fenstern_adapter::SourceUnavailable> {
        Ok(self.tage().max())
    }

    fn latest_before(&self, day: i64) -> Result<Option<i64>, fenstern_adapter::SourceUnavailable> {
        Ok(self.tage().filter(|tag| *tag < day).max())
    }
}

// --- Der Join: Fenster-Zeile + Bestand → das gelesene Aggregat ---------------------------

/// Die beiden **Lese**-Seams von `auswerten`, erfüllt über der committeten Fenster-Tabelle und dem
/// `obfuskiert`-Bestand — der Fake des Joins, den der echte io als SQL fährt. Er liegt **außerhalb**
/// der Transaktion (idempotentes Lesen).
pub(crate) struct FakeJoin<'a> {
    db: FakeDb,
    bestand: &'a FakeBestand,
}

impl FakeJoin<'_> {
    /// Setzt eine committete Fenster-Zeile mit ihren Buchungen zum gelesenen Aggregat zusammen.
    fn aggregat(&self, window: &fenstern_adapter::AnalysisWindowRow) -> AnalysisWindowRow {
        AnalysisWindowRow {
            currency: window.currency.clone(),
            seed_marker: SEED_MARKER.to_string(),
            bookings: window
                .booking_ids
                .iter()
                .filter_map(|id| self.bestand.zeile(id).cloned())
                .collect(),
        }
    }
}

impl AnalysisWindowSeam for FakeJoin<'_> {
    fn read(
        &self,
        window_id: &str,
    ) -> Result<Option<AnalysisWindowRow>, auswerten_adapter::SourceUnavailable> {
        Ok(self
            .db
            .committed
            .borrow()
            .windows
            .iter()
            .find(|(id, _)| id == window_id)
            .map(|(_, window)| self.aggregat(window)))
    }
}

impl OccurrenceSeam for FakeJoin<'_> {
    /// Die fenster-übergreifende Query: **alle** Fenster, die in der Lauf-Spanne liegen, mit ihren
    /// Buchungen — **eine** Query über viele Fenster, nicht N+1 Lesevorgänge (so wie der Seam-Vertrag
    /// es beschreibt). Die Spanne scopt auf **diesen** Lauf; Fenster früherer Läufe fallen heraus.
    fn bookings_in(
        &self,
        from: i64,
        to: i64,
    ) -> Result<Vec<WindowBookingRow>, auswerten_adapter::SourceUnavailable> {
        Ok(self
            .db
            .committed
            .borrow()
            .windows
            .iter()
            .filter(|(_, window)| from <= window.from && window.to <= to)
            .flat_map(|(_, window)| {
                window
                    .booking_ids
                    .iter()
                    .filter_map(|id| self.bestand.zeile(id).cloned())
            })
            .collect())
    }
}

// --- Kategorie-Register + Orakel --------------------------------------------------------

/// Das Kategorie-Register: hier **leer** — es kennt keine der Buchungsarten des Laufs, also fragt
/// die Domäne das Orakel. Genau so wird die Orakel-Verdrahtung im Lauf sichtbar.
#[derive(Default)]
pub(crate) struct FakeRegister;

impl CategoryRegistrySeam for FakeRegister {
    fn known_among(&self, _candidates: &[String]) -> Result<Vec<String>, RegistryUnavailable> {
        Ok(vec![])
    }
}

/// Das **Orakel** hinter dem `Kategorisierer`-Port (ADR-0004: die Domäne entscheidet, wann sie
/// fragt — das LLM ist kein Pipeline-Schritt). Es antwortet auf **jeden** Fingerabdruck mit
/// demselben Etikett; welche Fragen überhaupt gestellt werden, entscheidet die Domäne.
#[derive(Default)]
pub(crate) struct FakeOrakel {
    kategorie: &'static str,
}

impl FakeOrakel {
    pub(crate) fn antwortet(kategorie: &'static str) -> Self {
        Self { kategorie }
    }
}

impl CategorizerSeam for FakeOrakel {
    fn ask(
        &self,
        requests: &[CategoryRequestRow],
    ) -> Result<Vec<CategoryAnswerRow>, CategorizerUnavailable> {
        Ok(requests
            .iter()
            .map(|request| CategoryAnswerRow {
                fingerprint: request.fingerprint.clone(),
                category: self.kategorie.to_string(),
            })
            .collect())
    }
}

// --- Der Abnehmer der fenster-übergreifenden Reichweite ---------------------------------

/// Der **direkte Abnehmer** der zurückgegebenen Vorkommen: die zweite Reichweite legt **nichts** ab
/// (kein Vorkommens-/Fold-Log), ihr Ergebnis lebt allein in der Response. Hier sammelt es der Test
/// ein — später der Wiederkehr-Analyzer
/// ([0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)).
#[derive(Clone, Default)]
pub(crate) struct FakeSenke {
    empfangen: Rc<RefCell<Vec<Vec<OccurrenceRow>>>>,
}

impl FakeSenke {
    /// Die **Lieferungen** in ihrer Reihenfolge — je Zustellung eine. Bewusst nicht flachgeklopft:
    /// dass die Reichweite **einmal** läuft, ist Teil dessen, was der Test beobachtet.
    pub(crate) fn lieferungen(&self) -> Vec<Vec<OccurrenceRow>> {
        self.empfangen.borrow().clone()
    }
}

impl OccurrenceSink for FakeSenke {
    fn receive(&self, occurrences: Vec<OccurrenceRow>) {
        self.empfangen.borrow_mut().push(occurrences);
    }
}

// --- Fake-Arbeitseinheit: beide Ablagen + Outbox = die eine Unit ------------------------

/// Die geteilte, committete Wahrheit hinter allen Arbeitseinheiten — **beide** Ergebnis-Tabellen
/// und die Outbox.
#[derive(Clone, Default)]
pub(crate) struct FakeDb {
    committed: Rc<RefCell<Committed>>,
    fail_commit_ab: Option<usize>,
}

#[derive(Default)]
struct Committed {
    /// `fenstern`s Fenster-Tabelle: je Fenster die von der Ablage **neu vergebene** ID und die Zeile.
    windows: Vec<(String, fenstern_adapter::AnalysisWindowRow)>,
    /// `auswerten`s Ergebnis-Tabelle.
    evaluations: Vec<WindowEvaluationRow>,
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

    /// Die abgelegten **Pro-Fenster-Kennzahlen** — das erste der beiden Ergebnisse des Laufs.
    pub(crate) fn committed_evaluations(&self) -> Vec<WindowEvaluationRow> {
        self.committed.borrow().evaluations.clone()
    }

    /// Die committeten Nachrichten als `(message_type, partition_key)` — die Outbox, wie ein Broker
    /// sie sähe.
    pub(crate) fn committed_messages(&self) -> Vec<(String, String)> {
        self.committed.borrow().messages.clone()
    }

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

impl io::fenstern::UnitFactory for FakeDb {
    type Unit = FakeUnit;

    fn begin(&self) -> FakeUnit {
        FakeUnit::of(self)
    }
}

impl io::auswerten::UnitFactory for FakeDb {
    type Unit = FakeUnit;

    fn begin(&self) -> FakeUnit {
        FakeUnit::of(self)
    }
}

/// Eine Arbeitseinheit: **beide** gestageten Ablagen und die Outbox in einem. `save`/`publish`
/// stagen in eigene `pending`-Puffer; `commit` flusht sie in den geteilten `committed`-Sink. Ohne
/// Commit (Handler-Fehler) verwirft das Fallenlassen der Unit die `pending`-Arbeit.
pub(crate) struct FakeUnit {
    committed: Rc<RefCell<Committed>>,
    pending: RefCell<Committed>,
    fail_commit_ab: Option<usize>,
}

impl FakeUnit {
    fn of(db: &FakeDb) -> Self {
        Self {
            committed: Rc::clone(&db.committed),
            pending: RefCell::new(Committed::default()),
            fail_commit_ab: db.fail_commit_ab,
        }
    }
}

impl fenstern_adapter::AnalysisWindowStoreSeam for FakeUnit {
    fn save(
        &self,
        window: &fenstern_adapter::AnalysisWindowRow,
    ) -> Result<String, fenstern_adapter::StoreUnavailable> {
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

impl EvaluationStoreSeam for FakeUnit {
    fn save(
        &self,
        evaluation: &WindowEvaluationRow,
    ) -> Result<(), auswerten_adapter::StoreUnavailable> {
        self.pending
            .borrow_mut()
            .evaluations
            .push(evaluation.clone());
        Ok(())
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
        committed.evaluations.extend(pending.evaluations);
        committed.messages.extend(pending.messages);
        committed.commits += 1;
        Ok(())
    }
}

// --- Verdrahtung: vier Consumer, ein Dispatcher -----------------------------------------

/// Alles, was die Verdrahtung an **message-unabhängigen** Seams borgt — der Test hält es am Leben,
/// solange der Dispatcher lebt.
pub(crate) struct Verdrahtung<'a> {
    pub(crate) join: FakeJoin<'a>,
    pub(crate) register: FakeRegister,
    pub(crate) orakel: FakeOrakel,
    pub(crate) senke: FakeSenke,
}

impl<'a> Verdrahtung<'a> {
    pub(crate) fn neu(bestand: &'a FakeBestand, db: &FakeDb) -> Self {
        Self {
            join: FakeJoin {
                db: db.clone(),
                bestand,
            },
            register: FakeRegister,
            orakel: FakeOrakel::antwortet(KATEGORIE),
            senke: FakeSenke::default(),
        }
    }
}

/// Registriert **alle vier** Consumer der beiden Slices bei **einem** Dispatcher: `fenstern`s Anstoß
/// und Fenster-Schritt, `auswerten`s pro-Fenster- und fenster-übergreifende Reichweite. Vier
/// Kommandotypen, eine uniforme `message_type`-Auflösung — die Maschinerie brauchte dafür keine
/// Erweiterung.
pub(crate) fn dispatcher_mit<'a>(
    bestand: &'a FakeBestand,
    verdrahtung: &'a Verdrahtung<'a>,
    db: FakeDb,
) -> Dispatcher<'a> {
    let seitengröße = NonZeroU32::new(WINDOW_DAYS).expect("Seitengröße ist nicht null");
    let mut dispatcher = Dispatcher::new();
    for (message_type, consumer) in [
        (
            io::fenstern::START_MESSAGE_TYPE,
            Box::new(io::fenstern::start_consumer(bestand, db.clone()))
                as Box<dyn io::outbox::OutboxHandler + 'a>,
        ),
        (
            io::fenstern::CONSUMED_MESSAGE_TYPE,
            Box::new(io::fenstern::consumer(seitengröße, bestand, db.clone())),
        ),
        (
            io::auswerten::CONSUMED_MESSAGE_TYPE,
            Box::new(io::auswerten::consumer(
                &verdrahtung.join,
                &verdrahtung.register,
                &verdrahtung.orakel,
                eigene_konten(),
                db.clone(),
            )),
        ),
        (
            io::auswerten::CONSUMED_HISTORY_MESSAGE_TYPE,
            Box::new(io::auswerten::history_consumer(
                &verdrahtung.join,
                verdrahtung.senke.clone(),
                db,
            )),
        ),
    ] {
        assert!(
            dispatcher.register(message_type, consumer).is_none(),
            "frische Registrierung darf keinen Consumer verdrängen"
        );
    }
    dispatcher
}

pub(crate) fn envelope(message_type: &str, partition_key: &str) -> OutboxMessage {
    OutboxMessage {
        // Die Envelope-`id` ist eine io-vergebene Message-UUID — bewusst **verschieden** vom
        // Zeiger im `partition_key`, damit der Test den Handshake über `partition_key` erzwingt.
        id: format!("msg-uuid-{message_type}-{partition_key}"),
        message_type: message_type.to_string(),
        partition_key: partition_key.to_string(),
        content: String::new(),
        occurred_on_utc: 0,
        attempt: 0,
    }
}

/// Fährt den **ganzen Lauf über beide Slices**: den Anstoß hinein, dann **jede** committete
/// Nachricht wieder hinein — anders als in den Einzel-Slice-Tests gibt es hier keine „reist
/// stromabwärts"-Ausnahme mehr, denn stromabwärts steht jetzt `auswerten`. Bricht beim ersten
/// Fehler ab (der Broker würde dead-lettern bzw. retryen, statt die Kette fortzusetzen).
///
/// Die Schleife ist **bewusst gedeckelt** und **meldet sich laut**: terminierte die Kette nicht,
/// liefe der Regress sonst als `Ok(())` durch.
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test-Fixture: eine nicht terminierende Kette ist ein Regress und muss den Test hart \
              scheitern lassen, nicht als Err durch die Assertions reisen — der Rückgabewert gehört \
              allein der Disposition der Nachrichtenverarbeitung"
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
        offen.extend(nachschub.iter().map(|(typ, key)| envelope(typ, key)));
    }
    assert!(
        offen.is_empty(),
        "Broker-Schleife erreichte MAX_RUNDEN ({MAX_RUNDEN}) ohne Terminierung; noch {} Nachricht(en) offen",
        offen.len()
    );
    Ok(())
}

// --- Fixtures: der Bestand und die erwarteten Zeilen -------------------------------------

/// Das Etikett, das das Orakel auf jede unbekannte Buchungsart gibt.
pub(crate) const KATEGORIE: &str = "Abo";

/// Das Konto-Token des einen eigenen Kontos — so, wie der Loader es mit **demselben** Seed
/// tokenisiert hat wie `obfuskieren` die Gegenkonto-Token (Issue 0002).
const EIGENES_KONTO: &str = "KONTO_1";

/// Der Bestand des Laufs: er zerfällt bei Seitengröße 30 in genau **zwei** Fenster —
/// `[19753…19782]` mit der Februar-Buchung und der Januar-Umbuchung, `[19708…19737]` mit der
/// Januar-Buchung. Beide Fenster tragen **dieselbe** wiederkehrende Buchungsart: genau das kann die
/// pro-Fenster-Reichweite nicht sehen, und genau das legt die fenster-übergreifende zusammen.
pub(crate) fn bestand() -> FakeBestand {
    FakeBestand::mit(&[
        Buchung {
            id: "b-1",
            day: FEB_29,
            row: wiederkehrend(FEB_29, 1_999),
        },
        Buchung {
            id: "b-2",
            day: JAN_31,
            row: umbuchung(JAN_31, 250_000),
        },
        Buchung {
            id: "b-3",
            day: JAN_15,
            row: wiederkehrend(JAN_15, 1_999),
        },
    ])
}

/// Eine Buchung der **wiederkehrenden Art**: derselbe Händler, also derselbe Fingerabdruck — egal,
/// in welchem Fenster sie liegt.
fn wiederkehrend(booking_day: i64, cent: u64) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_cent: cent,
        direction: BookingDirection::Outgoing,
        recipient: BookingRecipient::Company {
            name: Some("Netflix".to_string()),
            creditor_id: HAENDLER.to_string(),
        },
        purpose: None,
        counter_account: None,
    }
}

/// Ein **Zufluss vom eigenen Konto** — eine Umbuchung: keine Gegenpartei (also kein Fingerabdruck),
/// aber ein Gegenkonto-**Token**, das die Auflösung gegen die Eigen-Menge einordnet.
fn umbuchung(booking_day: i64, cent: u64) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_cent: cent,
        direction: BookingDirection::Incoming,
        recipient: BookingRecipient::Absent,
        purpose: None,
        counter_account: Some(EIGENES_KONTO.to_string()),
    }
}

/// Die **token-orientierte Eigene-Konten-Config**, wie io sie beim Bau verdrahtet (ADR-0013,
/// Entscheidung F) — der Slice liest keine Datei.
pub(crate) fn eigene_konten() -> OwnAccountsConfig {
    OwnAccountsConfig {
        seed_marker: SEED_MARKER.to_string(),
        accounts: vec![OwnAccountRow {
            token: EIGENES_KONTO.to_string(),
            role: "giro".to_string(),
        }],
    }
}

/// Der **Fingerabdruck** der wiederkehrenden Art, so wie die Domäne ihn bildet — der Test schreibt
/// ihn selbst aus, statt ihn vom Slice zu erfragen: der Schlüssel ist Teil des beobachteten
/// Verhaltens, nicht ein Implementierungsdetail.
pub(crate) fn fingerabdruck() -> String {
    format!("COMPANY:{HAENDLER}")
}

pub(crate) fn monat(year: i64, month: i64, incoming: u64, outgoing: u64) -> MonthlyCashflowRow {
    MonthlyCashflowRow {
        year,
        month,
        incoming_cent: incoming,
        outgoing_cent: outgoing,
    }
}

/// Das erwartete **Pro-Fenster-Ergebnis** zu einem Fenster.
pub(crate) fn kennzahl(
    window_id: &str,
    months: Vec<MonthlyCashflowRow>,
    counter_accounts: Vec<CounterAccountRow>,
) -> WindowEvaluationRow {
    WindowEvaluationRow {
        window_id: window_id.to_string(),
        currency: "EUR".to_string(),
        months,
        fingerprints: vec![fingerabdruck()],
        counter_accounts,
        categories: vec![CategoryRow {
            fingerprint: fingerabdruck(),
            category: KATEGORIE.to_string(),
        }],
    }
}

/// Ein erwartetes **Vorkommen** auf der Zeitachse des Laufs.
pub(crate) fn vorkommen(booking_day: i64, cent: u64) -> OccurrenceRow {
    OccurrenceRow {
        fingerprint: fingerabdruck(),
        booking_day,
        amount_cent: cent,
        direction: BookingDirection::Outgoing,
    }
}

pub(crate) fn nachricht(message_type: &str, partition_key: &str) -> (String, String) {
    (message_type.to_string(), partition_key.to_string())
}
