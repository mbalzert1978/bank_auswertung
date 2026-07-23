//! Das reale transaktionale **Rückgrat** der Outbox über `rusqlite`
//! ([Issue 0052](../../../../docs/issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md),
//! [ADR-0014](../../../../docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)) — der Teil,
//! den die Build-Order bislang bewusst „vertagt, kein Issue" gehalten hat. Es ersetzt die
//! In-Memory-Doubles der io-privaten Outbox-Seams durch echte SQLite-Persistenz, **ohne** den
//! generischen [`Dispatcher`]/[`OutboxConsumer`](crate::OutboxConsumer) anzurühren.
//!
//! Produktiv läuft es **datei-gestützt** ([`SqliteOutbox::open`]) — nur eine Datei überdauert das
//! Prozess-Ende, und darauf ruht die Resume-Zusage über einen echten Absturz hinaus.
//! [`SqliteOutbox::open_in_memory`] ist die bewusst **flüchtige** Variante für Tests/ephemere Läufe.
//!
//! Der erste **slice-spezifische Ergebnis-Store** sitzt seit
//! [Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md) auf diesem
//! Fundament: [`EinlesenSqliteStore`](crate::einlesen::EinlesenSqliteStore) legt seine
//! `einlesen`-Tabelle über die crate-interne `SqliteOutbox::with_connection` an und stagt das
//! Zeilen-`INSERT` über `SqliteUnit::stage` in **dieselbe** committende Transaktion wie Ack und
//! `Ingested`-Publish. Dieses Modul selbst bleibt slice-blind — es liefert nur **Broker + UnitOfWork**
//! und die crate-internen Ablage-/Verbindungs-Primitive, auf die ein Store aufsetzt.

use std::cell::RefCell;
use std::rc::Rc;

use rusqlite::{Connection, Transaction};

use crate::config::AppConfig;
use crate::outbox::{OutboxCommand, OutboxError, OutboxMessage, OutboxPublisher, UnitOfWork};
use crate::{Dispatcher, MessageId};

/// Das SQLite-Outbox-Rückgrat: hält die Verbindung und die Outbox-Tabelle. Es ist zugleich der
/// **Broker** (liest offene Nachrichten, dispatcht sie über den bestehenden [`Dispatcher`]) und die
/// **Unit-Quelle** ([`begin`](SqliteOutbox::begin) öffnet je Nachricht eine [`SqliteUnit`]).
#[derive(Clone)]
pub struct SqliteOutbox {
    inner: Rc<Inner>,
}

struct Inner {
    /// Die geteilte Verbindung. **Serialitäts-Invariante:** alle Zugriffe laufen **strikt seriell** —
    /// [`deliver_open`](SqliteOutbox::deliver_open) verarbeitet eine Nachricht nach der anderen und
    /// gibt seine Lese-Leihgabe frei, **bevor** der Dispatch die schreibende Transaktion öffnet. Es
    /// gibt nie überlappende `RefCell`-Borrows und nie geschachtelte Transaktionen; nur so ist das
    /// `unchecked_transaction()` auf dieser Verbindung (in [`SqliteUnit::commit`]) korrekt. Der
    /// synchrone, single-user Zuschnitt ([ADR-0014](../../../../docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md))
    /// trägt diese Invariante.
    conn: RefCell<Connection>,
}

impl SqliteOutbox {
    /// Öffnet das **datei-gestützte** Rückgrat unter dem in der [`AppConfig`] konfigurierten Pfad
    /// (legt die Datei bei Bedarf an) und stellt die Outbox-Tabelle sicher (`IF NOT EXISTS` —
    /// bestehende Nachrichten bleiben erhalten).
    ///
    /// Das ist der **produktive** Einstieg: *wohin* persistiert wird, ist **konfigurierbar**
    /// ([`AppConfig::database_path`]), nicht im Code verdrahtet. Nur eine Datei überdauert das
    /// Prozess-Ende, und **darauf** ruht die At-least-once-/Resume-Zusage über einen echten Absturz
    /// hinaus — eine committete, aber noch offene Nachricht liegt beim nächsten Start weiterhin in der
    /// Tabelle und wird erneut zugestellt. `unixepoch`-Zeitstempel und `randomblob`-IDs vergibt
    /// SQLite selbst.
    pub fn open(config: &AppConfig) -> rusqlite::Result<Self> {
        let conn = Connection::open(config.database_path())?;
        Self::from_connection(conn)
    }

    /// Öffnet ein **flüchtiges** In-Memory-Rückgrat und legt die Outbox-Tabelle an. Bewusst
    /// **nicht dauerhaft** (die DB stirbt mit dem Prozess) — für Tests und ephemere Läufe; die
    /// überlebens-über-Neustart-Garantie trägt allein [`open`](SqliteOutbox::open).
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            inner: Rc::new(Inner {
                conn: RefCell::new(conn),
            }),
        })
    }

    /// Reiht eine Wurzel-Nachricht (Producer-Publish) in die Outbox ein und gibt ihre neue,
    /// DB-vergebene ID zurück — der Anstoß, den der Broker beim nächsten Lauf abholt.
    pub fn enqueue(&self, message_type: &str, partition_key: &str) -> rusqlite::Result<String> {
        self.inner.conn.borrow().query_row(
            INSERT_MESSAGE_RETURNING,
            (message_type, partition_key),
            |row| row.get(0),
        )
    }

    /// Reicht die geteilte Verbindung an `f` — der crate-interne Zugriff, über den ein Slice-Store
    /// seine Ergebnis-Tabelle **anlegt** (Schema, `IF NOT EXISTS`) und aus ihr **liest**. Die
    /// Outbox-Tabelle selbst rührt er nicht an; die schreibende Ablage einer verarbeiteten Nachricht
    /// läuft ausschließlich über die [`SqliteUnit`] und ihren Commit.
    pub(crate) fn with_connection<R>(
        &self,
        f: impl FnOnce(&Connection) -> rusqlite::Result<R>,
    ) -> rusqlite::Result<R> {
        f(&self.inner.conn.borrow())
    }

    /// Öffnet eine Arbeitseinheit für die Nachricht `ack`, die sie beim Commit als verarbeitet
    /// markiert. Eine **reine Funktion ihres Arguments**: die Ack-ID kommt explizit herein (nicht aus
    /// geteiltem Zustand), so ist zweifelsfrei, welche Nachricht die Unit ackt.
    pub fn begin(&self, ack: MessageId<'_>) -> SqliteUnit {
        SqliteUnit {
            inner: Rc::clone(&self.inner),
            message_id: ack.0.to_string(),
            staged: RefCell::new(Vec::new()),
        }
    }

    /// Liest die zum Aufrufzeitpunkt **offenen** Nachrichten (ein Schnappschuss) und dispatcht jede
    /// über den bestehenden [`Dispatcher`]. Die Ack-ID reist mit der Nachricht durch die generische
    /// Consumer-Maschinerie ([`OutboxConsumer`](crate::OutboxConsumer) → [`TransactionalHandler`](crate::TransactionalHandler)
    /// → [`begin`](SqliteOutbox::begin)); der Broker hält **keinen** ambient Zustand über die gerade
    /// laufende Nachricht.
    ///
    /// Scheitert der Commit einer Nachricht (Retry) oder ihr Handler terminal, bleibt sie **offen**
    /// und wird beim nächsten Lauf erneut zugestellt (at-least-once, resumierbar). Nachrichten, die ein
    /// Handler in diesem Lauf **neu** publiziert, holt erst der nächste Lauf ab (Schnappschuss-Semantik).
    ///
    /// **Redelivery ist heute unbegrenzt** (Fundament-Scope, Issue 0052): eine dauerhaft scheiternde
    /// Nachricht bleibt offen und wird jeden Lauf erneut zugestellt. Die attempt-basierte Eskalation
    /// Retry → Dead-Letter nach erschöpften Versuchen (Build-Order Schritt 5) ist **noch nicht gebaut**;
    /// die `attempt`-Spalte wird dafür bereits mitgeführt, aber von **keinem** Pfad erhöht oder
    /// honoriert. Wie `io` einen terminalen Fehlschlag surfacet, ist per
    /// [ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)
    /// vertagt.
    ///
    /// Gibt je Nachricht ihren Zustell-Ausgang zurück; die Reihenfolge folgt der Einreihung.
    pub fn deliver_open(
        &self,
        dispatcher: &Dispatcher,
    ) -> rusqlite::Result<Vec<(String, Result<(), OutboxError>)>> {
        let open = self.open_messages()?;
        let outcomes = open
            .into_iter()
            .map(|message| {
                let id = message.id.clone();
                let outcome = dispatcher.dispatch(message);
                (id, outcome)
            })
            .collect();
        Ok(outcomes)
    }

    /// Der Schnappschuss der offenen Nachrichten (`processed_on_utc IS NULL`) in Einreih-Reihenfolge.
    /// Die Leihgabe der Verbindung endet mit dieser Methode — der anschließende Dispatch öffnet die
    /// Verbindung für den schreibenden Commit frisch.
    fn open_messages(&self) -> rusqlite::Result<Vec<OutboxMessage>> {
        let conn = self.inner.conn.borrow();
        let mut statement = conn.prepare(SELECT_OPEN)?;
        let rows = statement.query_map([], |row| {
            Ok(OutboxMessage {
                id: row.get(0)?,
                message_type: row.get(1)?,
                partition_key: row.get(2)?,
                content: row.get(3)?,
                occurred_on_utc: row.get(4)?,
                attempt: row.get(5)?,
            })
        })?;
        rows.collect()
    }
}

/// Eine Arbeitseinheit über **einer** SQLite-Transaktion: Ack, Ablage und Publishes committen
/// atomar oder gar nicht.
pub struct SqliteUnit {
    inner: Rc<Inner>,
    /// Die ID der konsumierten Nachricht, die diese Einheit beim Commit ackt — explizit von
    /// [`begin`](SqliteOutbox::begin) gesetzt, nie aus geteiltem Zustand.
    message_id: String,
    staged: RefCell<Vec<StagedWrite>>,
}

type StagedWrite = Box<dyn FnOnce(&Transaction<'_>) -> rusqlite::Result<()>>;

impl SqliteUnit {
    /// Stellt eine Ablage-Schreibung für die committende Transaktion bereit — die generische
    /// Ablage-Primitive, auf die ein Slice-Store seine `INSERT`s legt. Seit
    /// [Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md) trägt sie den
    /// realen `einlesen`-Ergebnis-Store ([`EinlesenSqliteStore`](crate::einlesen::EinlesenSqliteStore)):
    /// dessen `store` stagt hierüber das Zeilen-`INSERT`, sodass Ablage, `Ingested`-Publish und Ack in
    /// **einer** Transaktion committen (der Backbone-Test treibt zusätzlich dieselbe Atomarität).
    pub(crate) fn stage<F>(&self, write: F)
    where
        F: FnOnce(&Transaction<'_>) -> rusqlite::Result<()> + 'static,
    {
        self.staged.borrow_mut().push(Box::new(write));
    }

    /// Reicht die geteilte Verbindung an `f` — der crate-interne Lese-/Prüf-Zugriff, über den ein
    /// Slice-Store seine ID **synchron** an der DB mintet und die Store-Verfügbarkeit prüft (der
    /// `INSERT` selbst wird über [`stage`](SqliteUnit::stage) in die committende Transaktion gelegt).
    /// **Nur außerhalb des Commit-Rands** aufrufen: der Handler läuft, bevor [`commit`](UnitOfWork::commit)
    /// die Verbindung für die schreibende Transaktion leiht (strikt serielle Verarbeitung), also
    /// überlappt diese Leihgabe nie mit jener.
    pub(crate) fn with_connection<R>(
        &self,
        f: impl FnOnce(&Connection) -> rusqlite::Result<R>,
    ) -> rusqlite::Result<R> {
        f(&self.inner.conn.borrow())
    }
}

impl OutboxPublisher for SqliteUnit {
    fn publish<T: OutboxCommand>(&self, command: T) {
        let message_type = T::MESSAGE_TYPE;
        let partition_key = command.partition_key().to_string();
        self.staged.borrow_mut().push(Box::new(move |tx| {
            tx.execute(INSERT_MESSAGE, (message_type, partition_key.as_str()))
                .map(|_| ())
        }));
    }
}

impl UnitOfWork for SqliteUnit {
    type Error = SqliteCommitError;

    /// Committet Ack + Ablage + Publishes in **einer** `rusqlite`-Transaktion: die konsumierte
    /// Nachricht wird als verarbeitet markiert, die gestageten Ablage-Schreibungen und die
    /// publizierten Outbox-Zeilen werden geschrieben, dann committet. Scheitert **irgendein**
    /// Schritt, rollt die fallengelassene [`Transaction`] alles zurück (Rollback bei `Drop`,
    /// [ADR-0014](../../../../docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)) — nichts
    /// davon persistiert, die Nachricht bleibt offen.
    ///
    /// **Idempotenter Ack (Effekt-genau-einmal):** der Ack trifft nur eine **noch offene** Zeile
    /// (`WHERE … AND processed_on_utc IS NULL`). Aktualisiert er `0` Zeilen — die Nachricht war schon
    /// verarbeitet, etwa durch einen überlappenden Zusteller auf derselben Datei —, bricht der Commit
    /// mit einem [`SqliteCommitError`] ab und **rollt die gestageten Seiteneffekte zurück**, statt sie
    /// doppelt zu committen. Das härtet das Outbox-Pattern über die (dokumentierte) strikt serielle
    /// Verarbeitung hinaus ab.
    fn commit(self) -> Result<(), SqliteCommitError> {
        let SqliteUnit {
            inner,
            message_id,
            staged,
        } = self;
        let staged = staged.into_inner();

        let conn = inner.conn.borrow();
        let transaction = conn.unchecked_transaction()?;
        let acked = transaction.execute(ACK_MESSAGE, (message_id.as_str(),))?;
        if acked != 1 {
            // Kein offener Treffer (`id` ist PRIMARY KEY, also nie > 1): bereits verarbeitet →
            // die Transaktion fällt hier ungecommittet und rollt die Seiteneffekte zurück.
            return Err(SqliteCommitError::already_processed());
        }
        staged
            .into_iter()
            .try_for_each(|write| write(&transaction))?;
        transaction.commit()?;
        Ok(())
    }
}

/// Ein gescheiterter Outbox-Commit. Die `Display` ist **redigiert** (reine Transaktions-Mechanik,
/// nie ein Roh-Zeilenwert). Ein Commit-Fehler wird zu [`OutboxError::Retry`](crate::outbox::OutboxError)
/// und die Nachricht **erneut zugestellt** — heute **unbegrenzt** (die attempt-basierte Eskalation
/// Retry → Dead-Letter ist noch nicht gebaut, Build-Order Schritt 5; wie `io` einen terminalen
/// Fehlschlag surfacet, ist per
/// [ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)
/// vertagt). **Sobald** diese Eskalation gebaut ist, wird diese `Display` nach erschöpften Versuchen
/// als Dead-Letter-Grund gespiegelt — darum ist die Redaktion jetzt schon Pflicht. Der rohe
/// `rusqlite`-Fehler wird **nur** im (Entwickler-)`Debug` gehalten, nie über `Display` oder `source`
/// nach außen gereicht.
#[derive(Debug)]
pub struct SqliteCommitError {
    // Privat: der Grund ist io-intern und darf nicht Teil der öffentlichen Fläche werden (er hält im
    // `RolledBack`-Fall den rohen `rusqlite`-Fehler — nur fürs Debug, nie über `Display`/`source`).
    kind: CommitErrorKind,
}

/// Warum ein Commit scheiterte — **io-intern**, nie über die redigierte `Display` nach außen.
#[derive(Debug)]
enum CommitErrorKind {
    /// Der schreibende Commit ist an SQLite gescheitert; die Transaktion wurde zurückgerollt.
    RolledBack {
        #[expect(
            dead_code,
            reason = "nur für die Entwickler-Diagnose im Debug gehalten — nie über Display oder \
                      source nach außen gereicht (Redaktions-Pflicht des Commit-Pfades)"
        )]
        source: rusqlite::Error,
    },
    /// Der Ack traf **keine offene** Nachricht (bereits verarbeitet / überlappende Zustellung durch
    /// einen zweiten Prozess auf derselben Datei). Zurückgerollt, damit keine Seiteneffekte doppelt
    /// landen — das Effekt-genau-einmal-Verhalten des Outbox-Patterns.
    AlreadyProcessed,
}

impl SqliteCommitError {
    /// Der Ack traf keine offene Nachricht — der Commit wurde übersprungen und zurückgerollt.
    const fn already_processed() -> Self {
        Self {
            kind: CommitErrorKind::AlreadyProcessed,
        }
    }
}

impl std::fmt::Display for SqliteCommitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            CommitErrorKind::RolledBack { .. } => {
                write!(
                    f,
                    "Outbox-Commit fehlgeschlagen (Transaktion zurückgerollt)"
                )
            }
            CommitErrorKind::AlreadyProcessed => write!(
                f,
                "Outbox-Commit übersprungen: Nachricht bereits verarbeitet (Transaktion zurückgerollt)"
            ),
        }
    }
}

// `source` bewusst **nicht** über `source()` gereicht: der rohe Fehler bleibt im `Debug`, aber die
// öffentliche Fehler-Kette gibt ihn nicht preis (Redaktions-Pflicht des Commit-Pfades).
impl std::error::Error for SqliteCommitError {}

impl From<rusqlite::Error> for SqliteCommitError {
    fn from(source: rusqlite::Error) -> Self {
        Self {
            kind: CommitErrorKind::RolledBack { source },
        }
    }
}

/// Das Outbox-Tabellen-Schema. `processed_on_utc IS NULL` markiert eine **offene** Nachricht.
///
/// **Partieller Index auf offene Nachrichten:** [`deliver_open`](SqliteOutbox::deliver_open) fragt
/// `WHERE processed_on_utc IS NULL` ab, und verarbeitete Zeilen werden (im Fundament-Scope) nie
/// gelöscht — ohne Index wäre das ein Full-Table-Scan pro Lauf über eine wachsende Tabelle. Der
/// **partielle** Index deckt genau dieses Prädikat und indiziert **nur** die (wenigen) offenen Zeilen,
/// bleibt also klein, während verarbeitete Zeilen anwachsen.
///
/// **`attempt` ist bewusst mitgeführt, aber im Fundament-Scope (Issue 0052) ungenutzt:** die Spalte
/// hält im Envelope den Zustellversuchs-Zähler bereit, den erst die attempt-basierte
/// Retry → Dead-Letter-Eskalation (Build-Order Schritt 5) erhöht und honoriert. Bis dahin bleibt sie
/// auf `0`, und Redelivery ist unbegrenzt (s. [`SqliteOutbox::deliver_open`]).
/// `created_at`/`updated_at` (Epoch-Millis) tragen den Row-Audit-Zeitstempel jeder Nachricht
/// (`.rules/sql/schema-konventionen.md`) — bewusst getrennt vom fachlichen `occurred_on_utc`
/// (Auftrittszeitpunkt der Nachricht) und dem Ack-Marker `processed_on_utc`: `created_at` hält die
/// Einfüge-Zeit, `updated_at` wird beim Ack mitgezogen (s. `ACK_MESSAGE`), damit die letzte
/// Zeilen-Änderung rekonstruierbar bleibt.
const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS outbox (
    id              TEXT    PRIMARY KEY,
    message_type    TEXT    NOT NULL,
    partition_key   TEXT    NOT NULL,
    content         TEXT    NOT NULL DEFAULT '',
    occurred_on_utc INTEGER NOT NULL,
    attempt         INTEGER NOT NULL DEFAULT 0,
    processed_on_utc INTEGER,
    created_at      INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at      INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
);
CREATE INDEX IF NOT EXISTS outbox_open_idx
    ON outbox (processed_on_utc) WHERE processed_on_utc IS NULL;";

const INSERT_MESSAGE: &str = "\
INSERT INTO outbox (id, message_type, partition_key, occurred_on_utc)
VALUES (lower(hex(randomblob(16))), ?1, ?2, CAST(unixepoch('subsec') * 1000 AS INTEGER));";

const INSERT_MESSAGE_RETURNING: &str = "\
INSERT INTO outbox (id, message_type, partition_key, occurred_on_utc)
VALUES (lower(hex(randomblob(16))), ?1, ?2, CAST(unixepoch('subsec') * 1000 AS INTEGER))
RETURNING id;";

const SELECT_OPEN: &str = "\
SELECT id, message_type, partition_key, content, occurred_on_utc, attempt
FROM outbox
WHERE processed_on_utc IS NULL
ORDER BY rowid;";

/// Ackt **nur eine noch offene** Nachricht (`AND processed_on_utc IS NULL`). Der Zusatz macht den
/// Ack **idempotent**: trifft er keine offene Zeile (bereits verarbeitet / überlappende Zustellung
/// durch einen zweiten Prozess auf derselben Datei), aktualisiert er `0` Zeilen — und der Commit von
/// [`SqliteUnit`] scheitert, statt still Seiteneffekte doppelt zu committen.
const ACK_MESSAGE: &str = "\
UPDATE outbox
SET processed_on_utc = CAST(unixepoch('subsec') * 1000 AS INTEGER),
    updated_at       = CAST(unixepoch('subsec') * 1000 AS INTEGER)
WHERE id = ?1 AND processed_on_utc IS NULL;";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OutboxConsumer, TransactionError, TransactionalHandler, run_in_unit};
    use application_kernel::CommandDecoder;
    use std::cell::Cell;

    const ROOT_TYPE: &str = "Root";
    const NEXT_TYPE: &str = "Next";

    struct Cmd;

    struct RootDecoder;
    impl CommandDecoder for RootDecoder {
        type Command = Cmd;
        fn decode(_partition_key: &str, _content: &str) -> Option<Cmd> {
            Some(Cmd)
        }
    }

    #[derive(Debug)]
    struct TestError;
    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "redigierter testfehler")
        }
    }
    impl std::error::Error for TestError {}

    /// Ein stromabwärtiges Kommando, das der Test-Handler publiziert.
    struct NextCommand;
    impl OutboxCommand for NextCommand {
        const MESSAGE_TYPE: &'static str = NEXT_TYPE;
        fn partition_key(&self) -> &str {
            "next-key"
        }
    }

    /// Der Slice-Stellvertreter: stellt je Nachricht eine Ablage-Zeile bereit **und** publiziert eine
    /// stromabwärtige Nachricht, beide in der committenden Unit. Zählt seine Aufrufe (für den
    /// Redelivery-Nachweis).
    struct RecordingTx {
        outbox: SqliteOutbox,
        ablage_note: String,
        calls: Rc<Cell<u32>>,
        /// Anzahl der ersten Aufrufe, deren Commit **absichtlich** scheitert (simuliert einen
        /// Absturz vor dem Ack): der Handler legt zusätzlich eine Schreibung in eine nicht
        /// existierende Tabelle, sodass mitten im Commit ein echter SQLite-Fehler entsteht.
        poison: Rc<Cell<u32>>,
    }

    impl TransactionalHandler for RecordingTx {
        type Command = Cmd;
        type Decoder = RootDecoder;
        type Error = TestError;

        fn handle(
            &self,
            ack: MessageId<'_>,
            _command: Cmd,
        ) -> Result<(), TransactionError<TestError>> {
            self.calls.set(self.calls.get() + 1);
            let poisoned = self.poison.get() > 0;
            if poisoned {
                self.poison.set(self.poison.get() - 1);
            }
            let note = self.ablage_note.clone();
            run_in_unit(self.outbox.begin(ack), move |unit, _publishing| {
                unit.stage(move |tx| {
                    tx.execute("INSERT INTO results (note) VALUES (?1);", (note.as_str(),))
                        .map(|_| ())
                });
                unit.publish(NextCommand);
                if poisoned {
                    unit.stage(|tx| {
                        tx.execute("INSERT INTO does_not_exist (x) VALUES (1);", ())
                            .map(|_| ())
                    });
                }
                Ok::<(), TestError>(())
            })
        }
    }

    /// Verdrahtet den Test-Handler als [`OutboxConsumer`] unter [`ROOT_TYPE`].
    fn dispatcher_mit(consumer: OutboxConsumer<RecordingTx>) -> Dispatcher<'static> {
        let mut dispatcher = Dispatcher::new();
        assert!(
            dispatcher.register(ROOT_TYPE, Box::new(consumer)).is_none(),
            "frische Registrierung darf keinen Consumer verdrängen"
        );
        dispatcher
    }

    fn results_count(outbox: &SqliteOutbox, note: &str) -> rusqlite::Result<u32> {
        outbox.inner.conn.borrow().query_row(
            "SELECT count(*) FROM results WHERE note = ?1;",
            (note,),
            |row| row.get(0),
        )
    }

    fn published_of_type(outbox: &SqliteOutbox, message_type: &str) -> rusqlite::Result<u32> {
        outbox.inner.conn.borrow().query_row(
            "SELECT count(*) FROM outbox WHERE message_type = ?1;",
            (message_type,),
            |row| row.get(0),
        )
    }

    fn is_processed(outbox: &SqliteOutbox, id: &str) -> rusqlite::Result<bool> {
        outbox.inner.conn.borrow().query_row(
            "SELECT processed_on_utc IS NOT NULL FROM outbox WHERE id = ?1;",
            (id,),
            |row| row.get(0),
        )
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert_eq! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ack_ablage_und_publish_committen_atomar() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        outbox
            .inner
            .conn
            .borrow()
            .execute_batch("CREATE TABLE results (note TEXT NOT NULL);")?;
        let root = outbox.enqueue(ROOT_TYPE, "root-key")?;

        let calls = Rc::new(Cell::new(0));
        let consumer = OutboxConsumer::new(RecordingTx {
            outbox: outbox.clone(),
            ablage_note: "abgelegt".to_string(),
            calls: Rc::clone(&calls),
            poison: Rc::new(Cell::new(0)),
        });
        let dispatcher = dispatcher_mit(consumer);

        outbox.deliver_open(&dispatcher)?;

        // Ack: die konsumierte Wurzel-Nachricht ist als verarbeitet markiert.
        assert!(is_processed(&outbox, &root)?, "die Wurzel muss geackt sein");
        // Ablage: die Ergebnis-Zeile ist committet.
        assert_eq!(
            results_count(&outbox, "abgelegt")?,
            1,
            "Ablage muss committet sein"
        );
        // Publish: die stromabwärtige Nachricht liegt in der Outbox.
        assert_eq!(
            published_of_type(&outbox, NEXT_TYPE)?,
            1,
            "die publizierte Nachricht muss in der Outbox liegen"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert_eq! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ein_fehlschlag_rollt_ack_ablage_und_publish_gemeinsam_zurueck() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        // `note` ist UNIQUE und mit „belegt" vorbesetzt — die Ablage des Handlers kollidiert beim
        // Flush, sodass mitten im Commit ein echter SQLite-Fehler entsteht.
        outbox
            .inner
            .conn
            .borrow()
            .execute_batch("CREATE TABLE results (note TEXT NOT NULL UNIQUE);")?;
        outbox
            .inner
            .conn
            .borrow()
            .execute("INSERT INTO results (note) VALUES ('belegt');", ())?;
        let root = outbox.enqueue(ROOT_TYPE, "root-key")?;

        let calls = Rc::new(Cell::new(0));
        let consumer = OutboxConsumer::new(RecordingTx {
            outbox: outbox.clone(),
            ablage_note: "belegt".to_string(),
            calls: Rc::clone(&calls),
            poison: Rc::new(Cell::new(0)),
        });
        let dispatcher = dispatcher_mit(consumer);

        let outcomes = outbox.deliver_open(&dispatcher)?;

        // Der Commit scheitert → io-interner Retry (der einzige Retry-Ursprung); die Nachricht bleibt
        // liegen, damit sie erneut zugestellt wird.
        assert!(
            matches!(
                outcomes.as_slice(),
                [(id, Err(OutboxError::Retry { .. }))] if id == &root
            ),
            "ein gescheiterter Commit muss zu Retry führen: {outcomes:?}"
        );
        // Alle drei rollen gemeinsam zurück: die Wurzel bleibt offen (nicht geackt) …
        assert!(
            !is_processed(&outbox, &root)?,
            "die Wurzel darf nicht geackt sein"
        );
        // … keine stromabwärtige Nachricht wurde publiziert …
        assert_eq!(
            published_of_type(&outbox, NEXT_TYPE)?,
            0,
            "kein Publish darf den Rollback überleben"
        );
        // … und die Ablage-Tabelle trägt nur die vorbesetzte Zeile.
        assert_eq!(
            results_count(&outbox, "belegt")?,
            1,
            "keine Ablage-Schreibung darf den Rollback überleben"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert_eq! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_vor_dem_ack_abgestuerzte_nachricht_wird_erneut_zugestellt() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        outbox
            .inner
            .conn
            .borrow()
            .execute_batch("CREATE TABLE results (note TEXT NOT NULL);")?;
        // Producer-Commit: die Nachricht liegt committet und offen in der Outbox.
        let root = outbox.enqueue(ROOT_TYPE, "root-key")?;

        let calls = Rc::new(Cell::new(0));
        let consumer = OutboxConsumer::new(RecordingTx {
            outbox: outbox.clone(),
            ablage_note: "abgelegt".to_string(),
            calls: Rc::clone(&calls),
            // Der **erste** Zustellversuch stürzt vor dem Ack ab (Commit scheitert).
            poison: Rc::new(Cell::new(1)),
        });
        let dispatcher = dispatcher_mit(consumer);

        // Lauf 1: zugestellt, aber der Commit scheitert → Retry, nichts persistiert, Nachricht offen.
        let erster_lauf = outbox.deliver_open(&dispatcher)?;
        assert!(
            matches!(
                erster_lauf.as_slice(),
                [(id, Err(OutboxError::Retry { .. }))] if id == &root
            ),
            "der abgestürzte Versuch muss als Retry erscheinen: {erster_lauf:?}"
        );
        assert!(
            !is_processed(&outbox, &root)?,
            "vor dem Ack abgestürzt ⇒ die Nachricht bleibt offen"
        );
        assert_eq!(calls.get(), 1, "genau ein erster Zustellversuch");

        // Lauf 2 (Resume): dieselbe offene Nachricht wird **erneut** zugestellt und diesmal geackt.
        let zweiter_lauf = outbox.deliver_open(&dispatcher)?;
        assert!(
            matches!(zweiter_lauf.as_slice(), [(id, Ok(()))] if id == &root),
            "der Resume-Lauf muss dieselbe Nachricht erneut zustellen und ackt sie: {zweiter_lauf:?}"
        );
        assert_eq!(
            calls.get(),
            2,
            "die Nachricht wurde erneut zugestellt (at-least-once)"
        );
        assert!(
            is_processed(&outbox, &root)?,
            "nach dem Resume ist die Wurzel geackt"
        );
        assert_eq!(
            results_count(&outbox, "abgelegt")?,
            1,
            "die Ablage persistiert erst mit dem erfolgreichen Resume-Commit"
        );
        assert_eq!(
            published_of_type(&outbox, NEXT_TYPE)?,
            1,
            "die stromabwärtige Nachricht persistiert erst mit dem Resume-Commit"
        );
        Ok(())
    }

    /// RAII-Wächter über einen eindeutigen temporären DB-Pfad: löscht die Datei beim Fallenlassen —
    /// auch wenn ein `assert!` den Test abbricht.
    struct TempDbPath(std::path::PathBuf);

    impl TempDbPath {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|dauer| dauer.as_nanos())
                .unwrap_or(0);
            let mut pfad = std::env::temp_dir();
            pfad.push(format!(
                "bank_auswertung_outbox_{tag}_{}_{nanos}.sqlite",
                std::process::id()
            ));
            Self(pfad)
        }
    }

    impl Drop for TempDbPath {
        fn drop(&mut self) {
            // Best-effort-Aufräumen: eine bereits fehlende Datei ist kein Fehler.
            match std::fs::remove_file(&self.0) {
                Ok(()) | Err(_) => {}
            }
        }
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_offene_nachricht_ueberlebt_einen_prozess_neustart() -> rusqlite::Result<()> {
        let db = TempDbPath::new("restart");
        // Der DB-Pfad kommt aus der App-Konfiguration, nicht aus dem Code.
        let config = AppConfig::new(db.0.clone());

        // Erster Prozess: eine Nachricht einreihen, dann Verbindung schließen (= „Prozess-Ende").
        let root = {
            let outbox = SqliteOutbox::open(&config)?;
            outbox.enqueue(ROOT_TYPE, "root-key")?
        };

        // Neustart: dieselbe Config, dieselbe Datei — die offene Nachricht liegt noch da.
        let outbox = SqliteOutbox::open(&config)?;
        outbox
            .inner
            .conn
            .borrow()
            .execute_batch("CREATE TABLE IF NOT EXISTS results (note TEXT NOT NULL);")?;
        assert!(
            !is_processed(&outbox, &root)?,
            "die eingereihte Nachricht überlebt den Neustart offen in der Datei"
        );

        let calls = Rc::new(Cell::new(0));
        let consumer = OutboxConsumer::new(RecordingTx {
            outbox: outbox.clone(),
            ablage_note: "abgelegt".to_string(),
            calls: Rc::clone(&calls),
            poison: Rc::new(Cell::new(0)),
        });
        let dispatcher = dispatcher_mit(consumer);

        // Nach dem Neustart holt der Broker die offene Nachricht ab und ackt sie (resumierbar).
        let outcomes = outbox.deliver_open(&dispatcher)?;
        assert!(
            matches!(outcomes.as_slice(), [(id, Ok(()))] if id == &root),
            "nach dem Neustart wird die offene Nachricht zugestellt: {outcomes:?}"
        );
        assert!(is_processed(&outbox, &root)?, "und ist nun geackt");
        assert_eq!(calls.get(), 1, "genau einmal zugestellt");
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ein_ack_auf_eine_bereits_verarbeitete_nachricht_rollt_seiteneffekte_zurueck()
    -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        outbox
            .inner
            .conn
            .borrow()
            .execute_batch("CREATE TABLE results (note TEXT NOT NULL);")?;
        let root = outbox.enqueue(ROOT_TYPE, "root-key")?;
        // Ein überlappender Zusteller hat die Nachricht bereits verarbeitet (Ack gesetzt).
        outbox.inner.conn.borrow().execute(
            "UPDATE outbox SET processed_on_utc = 1 WHERE id = ?1;",
            (root.as_str(),),
        )?;

        // Eine zweite Einheit, die dieselbe Nachricht erneut ackt, darf ihre Seiteneffekte **nicht**
        // committen — der idempotente Ack trifft keine offene Zeile.
        let unit = outbox.begin(MessageId(&root));
        unit.stage(|tx| {
            tx.execute("INSERT INTO results (note) VALUES ('doppelt');", ())
                .map(|_| ())
        });
        unit.publish(NextCommand);
        let ergebnis = unit.commit();

        assert!(
            matches!(
                ergebnis,
                Err(SqliteCommitError {
                    kind: CommitErrorKind::AlreadyProcessed
                })
            ),
            "der Ack traf keine offene Nachricht → Commit muss als AlreadyProcessed abbrechen: {ergebnis:?}"
        );
        // Alles rollte zurück: weder die gestagete Ablage noch die stromabwärtige Nachricht landeten.
        assert_eq!(
            results_count(&outbox, "doppelt")?,
            0,
            "keine doppelte Ablage darf committet werden"
        );
        assert_eq!(
            published_of_type(&outbox, NEXT_TYPE)?,
            0,
            "kein doppelter Publish darf committet werden"
        );
        Ok(())
    }
}
