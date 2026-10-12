//! Das reale transaktionale **Rückgrat** der Outbox über `rusqlite`
//! ([Issue 0052](../../../../docs/issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md),
//! [ADR-0014](../../../../docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)) — der Teil,
//! den die Build-Order bislang bewusst „vertagt, kein Issue" gehalten hat. Es ersetzt die
//! In-Memory-Doubles der io-privaten Outbox-Seams durch echte SQLite-Persistenz, **ohne** den
//! generischen [`Dispatcher`]/[`OutboxConsumer`](crate::OutboxConsumer) anzurühren.
//!
//! Produktiv läuft es **datei-gestützt** — nur eine Datei überdauert das Prozess-Ende, und darauf
//! ruht die Resume-Zusage über einen echten Absturz hinaus. **Geöffnet wird es nicht hier**, sondern
//! über den Schema-Besitzer [`schema::open`](crate::schema::open)
//! ([Issue 0084](../../../../docs/issues/0084-schema-an-einem-ort-und-versionsstempel-als-migrationsachse.md)):
//! er legt das Schema **aller** Tabellen an **einer** Stelle an, stempelt die Datei mit der
//! Schema-Version und weist eine Datei abweichender Version ab. Dieses Modul selbst legt **kein**
//! Schema an — es steuert nur die Outbox-Tabelle als [`TableSpec`] bei.
//!
//! Der erste **slice-spezifische Ergebnis-Store** sitzt seit
//! [Issue 0053](../../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md) auf diesem
//! Fundament: [`EinlesenSqliteStore`](crate::einlesen::EinlesenSqliteStore) stagt das
//! Zeilen-`INSERT` in **dieselbe** committende Transaktion wie Ack und `Ingested`-Publish. Dieses
//! Modul selbst bleibt slice-blind — es liefert nur **Broker + UnitOfWork** und die crate-internen
//! Ablage-/Verbindungs-Primitive, auf die ein Store aufsetzt.
//!
//! Das **Skelett über diesen Primitiven** (Store-Struct, `new`, Unit-Newtype, Preflight, Minten,
//! Outbox-Passthrough) trägt seit
//! [Issue 0074](../../../../docs/issues/0074-storespec-abstraktion-einf-hren-und-einlesen-migrieren.md)
//! der generische [`SliceStore`]/[`SliceUnit`] **einmal**, parametrisiert über [`StoreSpec`]
//! ([ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md)). Seit dem
//! `auswerten`-Capstone ([Issue 0077](../../../../docs/issues/0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md))
//! laufen **alle vier** Stores darüber — `einlesen` (eine Tabelle), `obfuskierung`, `fenstern` (zwei)
//! und `auswerten` (fünf). Die transitorischen Helfer daneben (`mint_id`, `SqliteUnit::stage` und das
//! `sqlite_backed_unit!`-Makro, das den Outbox-Passthrough je Store erzeugte) sind damit ersatzlos
//! entfallen: der Passthrough lebt **einmal** generisch auf [`SliceUnit`], Preflight und Minten in
//! dessen `store_dto`. Mit dem Versionsstempel (Issue 0084) ist auch `ensure_columns` entfallen, die
//! stille Audit-Spalten-Nachrüstung auf Alt-Dateien: eine Datei ohne passenden Stempel wird jetzt
//! abgewiesen, statt still umgebaut — die Achse für eine Schema-Änderung ist die Version.

mod delivery;
mod slice_store;

pub use delivery::{DeadLetter, Delivery, MAX_DELIVERY_ATTEMPTS};
pub use slice_store::{MintedId, SliceStore, SliceUnit, StagedRow, StoreSpec, TableSpec};

use std::cell::RefCell;
use std::rc::Rc;

use rusqlite::{Connection, OptionalExtension, params};

use crate::outbox::{OutboxCommand, OutboxMessage, OutboxPublisher, UnitOfWork};
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
    /// Hüllt eine **bereits geöffnete** Verbindung in das Rückgrat — **ohne** ein Schema anzulegen.
    /// Außerhalb von Tests ruft das allein der Schema-Besitzer [`schema::open`](crate::schema::open):
    /// er hat die Datei zuvor angelegt bzw. ihre Version geprüft, erst danach entsteht hier das
    /// Rückgrat darüber.
    pub(crate) fn from_connection(conn: Connection) -> Self {
        Self {
            inner: Rc::new(Inner {
                conn: RefCell::new(conn),
            }),
        }
    }

    /// Öffnet ein **flüchtiges** In-Memory-Rückgrat mit **nur** der Outbox-Tabelle — der **Test-Weg**
    /// für die Rückgrat- und Store-Tests, die ihre Ergebnis-Tabellen gezielt selbst anlegen (oder
    /// gerade nicht). Eine echte Datenbank bekommt ihr Schema ausschließlich über
    /// [`schema::open`](crate::schema::open).
    #[cfg(test)]
    pub(crate) fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(OUTBOX_TABLE.ddl)?;
        Ok(Self::from_connection(conn))
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

    /// Reicht die geteilte Verbindung an `f` — der crate-interne Zugriff, über den ein Lese-Seam aus
    /// einer Ergebnis-Tabelle **liest** (und ein Test sich gezielt Tabellen anlegt). Die
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
    /// **Was mit einer gescheiterten Nachricht geschieht, entscheidet hier der Broker**
    /// ([Issue 0085](../../../../docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md)
    /// beantwortet die in
    /// [ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)
    /// vertagte Frage) — je Nachricht, gleich nach ihrem Versuch, siehe [`Delivery`]:
    ///
    /// - **terminal** (`Warning` — der Slice scheiterte; `Critical` — die Hülle trug nicht): die
    ///   Nachricht wird **sofort** ausgesondert, mit dem redigierten Grund.
    /// - **transient** (`Retry` — der Commit scheiterte): sie bleibt offen und wird erneut zugestellt,
    ///   bis [`MAX_DELIVERY_ATTEMPTS`] Versuche verbraucht sind; dann wird auch sie ausgesondert.
    ///
    /// Eine ausgesonderte Nachricht ist nicht mehr offen: kein späterer Schnappschuss liefert sie
    /// wieder, und die Nachrichten hinter ihr laufen weiter. Der Slice merkt davon nichts — er gibt
    /// weiterhin nur seinen nackten Domänen-Fehler zurück.
    ///
    /// Nachrichten, die ein Handler in diesem Aufruf **neu** publiziert, holt erst der nächste Aufruf ab
    /// (Schnappschuss-Semantik).
    ///
    /// Gibt je Nachricht **den zugestellten Envelope samt ihrem Ausgang** zurück; die Reihenfolge folgt
    /// der Einreihung. Der Envelope reist mit, weil der Aufrufer sonst nur eine opake ID hätte: das
    /// **Lauf-Protokoll** ([`run`](crate::run)) zählt je `message_type`, und dieser Typ ist ein
    /// statischer Dispatch-Key, kein Rohwert.
    ///
    /// **Kein verbuchter Ausgang geht verloren:** lässt sich ein Fehlschlag nicht verbuchen, steht das
    /// als [`Delivery::Unrecorded`] **neben** den schon verbuchten Ausgängen dieses Aufrufs, und der
    /// Broker stellt danach nichts mehr zu (die übrigen Nachrichten bleiben offen) — statt alles
    /// Verbuchte mit einem Fehler zu verwerfen.
    ///
    /// # Errors
    ///
    /// Ein `rusqlite`-Fehler, wenn sich die offenen Nachrichten gar nicht erst lesen lassen.
    pub fn deliver_open(
        &self,
        dispatcher: &Dispatcher,
    ) -> rusqlite::Result<Vec<(OutboxMessage, Delivery)>> {
        let mut deliveries = Vec::new();
        for message in self.open_messages()? {
            let outcome = dispatcher.dispatch(message.clone());
            let delivery = self.settle(&message, outcome);
            let unrecorded = matches!(delivery, Delivery::Unrecorded { .. });
            deliveries.push((message, delivery));
            if unrecorded {
                break;
            }
        }
        Ok(deliveries)
    }

    /// Der Schnappschuss der offenen Nachrichten (`status = 'open'`) in Einreih-Reihenfolge.
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

    /// Der `partition_key` der **ältesten noch offenen** Nachricht dieses Typs, falls es eine gibt.
    ///
    /// Schmaler als [`open_messages`](Self::open_messages) und mit anderer Absicht: nicht „was ist zu
    /// tun", sondern „liegt hier noch etwas von vorher". [`run`](crate::run) fragt damit **vor** dem
    /// Einreihen, ob ein Auslöser einer anderen Quelle unerledigt herumliegt — der einzige Weg, den
    /// stillen Doppelimport zu bemerken, bevor er passiert.
    pub(crate) fn pending_key(&self, message_type: &str) -> rusqlite::Result<Option<String>> {
        let conn = self.inner.conn.borrow();
        conn.query_row(SELECT_PENDING_KEY, params![message_type], |row| row.get(0))
            .optional()
    }
}

/// Eine Arbeitseinheit über **einer** SQLite-Transaktion: Ack, Ablage und Publishes committen
/// atomar oder gar nicht.
pub struct SqliteUnit {
    inner: Rc<Inner>,
    /// Die ID der konsumierten Nachricht, die diese Einheit beim Commit ackt — explizit von
    /// [`begin`](SqliteOutbox::begin) gesetzt, nie aus geteiltem Zustand.
    message_id: String,
    staged: RefCell<Vec<StagedRow>>,
}

impl SqliteUnit {
    /// Stellt eine Ablage-Schreibung für die committende Transaktion bereit — die **eine** generische
    /// Ablage-Primitive, auf die jeder Slice-Store seine `INSERT`s legt. Sie nimmt die Schreibung
    /// **schon geboxt** ([`StagedRow`]) entgegen: so liefert sie die [`SliceUnit`], deren
    /// [`StoreSpec`] die Row baut — kein zweites Boxen. Was hierüber gestaget wird, committet mit dem
    /// Ack und den Publishes derselben Einheit oder gar nicht.
    pub(crate) fn stage_row(&self, write: StagedRow) {
        self.staged.borrow_mut().push(write);
    }

    /// Reicht die geteilte Verbindung an `f` — der crate-interne Lese-/Prüf-Zugriff, über den ein
    /// Slice-Store seine ID **synchron** an der DB mintet und die Store-Verfügbarkeit prüft (der
    /// `INSERT` selbst wird über [`stage_row`](SqliteUnit::stage_row) in die committende Transaktion
    /// gelegt).
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
    /// Schritt, rollt die fallengelassene [`rusqlite::Transaction`] alles zurück (Rollback bei `Drop`,
    /// [ADR-0014](../../../../docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)) — nichts
    /// davon persistiert, die Nachricht bleibt offen.
    ///
    /// **Idempotenter Ack (Effekt-genau-einmal):** der Ack trifft nur eine **noch offene** Zeile
    /// (`WHERE … AND status = 'open'`). Aktualisiert er `0` Zeilen — die Nachricht war schon
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
/// und die Nachricht **erneut zugestellt**, bis [`MAX_DELIVERY_ATTEMPTS`] Versuche verbraucht sind;
/// dann wird sie ausgesondert, und diese `Display` liegt als Aussonderungs-Grund an ihrer Outbox-Zeile
/// ([Issue 0085](../../../../docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md))
/// — darum ist die Redaktion Pflicht. Der rohe
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

/// Das Outbox-Tabellen-Schema. Die **`status`-Spalte** trägt, wo eine Nachricht steht
/// ([Issue 0085](../../../../docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md)):
/// `open` (wird zugestellt), `processed` (geackt) oder `dead_letter` (aus der Zustellung genommen).
/// Nur `open` wird zugestellt — ausgesonderte Nachrichten bleiben als Zeile liegen und sind über
/// [`SqliteOutbox::dead_letters`] abfragbar, statt verloren zu gehen.
///
/// **Illegale Kombinationen sind unkonstruierbar** (`.rules/sql/schema-konventionen.md`, ADR-0005):
/// `processed_on_utc` steht genau bei `processed`, `dead_letter_reason` genau bei `dead_letter` — je
/// ein Tabellen-`CHECK`. Eine geackte Nachricht mit Aussonderungs-Grund oder eine offene mit Ack-Stempel
/// lehnt SQLite ab.
///
/// **`attempt`** zählt die Zustellversuche: jeder Ack und jeder gescheiterte Versuch zieht ihn hoch
/// (s. [`SqliteOutbox::deliver_open`]). An ihm entscheidet der Broker, wann ein **transienter**
/// Fehlschlag die Nachricht aus der Zustellung nimmt ([`MAX_DELIVERY_ATTEMPTS`]).
///
/// **Partieller Index auf offene Nachrichten:** [`deliver_open`](SqliteOutbox::deliver_open) fragt
/// `WHERE status = 'open'` ab, und verarbeitete Zeilen werden nie gelöscht — ohne Index wäre das ein
/// Full-Table-Scan pro Lauf über eine wachsende Tabelle. Der **partielle** Index deckt genau dieses
/// Prädikat und indiziert **nur** die (wenigen) offenen Zeilen.
///
/// `created_at`/`updated_at` (Epoch-Millis) tragen den Row-Audit-Zeitstempel jeder Nachricht
/// (`.rules/sql/schema-konventionen.md`) — bewusst getrennt vom fachlichen `occurred_on_utc`
/// (Auftrittszeitpunkt der Nachricht) und dem Ack-Marker `processed_on_utc`: `created_at` hält die
/// Einfüge-Zeit, `updated_at` wird bei jedem Statuswechsel und jedem gezählten Versuch mitgezogen.
const OUTBOX_DDL: &str = "\
CREATE TABLE IF NOT EXISTS outbox (
    id                 TEXT    PRIMARY KEY,
    message_type       TEXT    NOT NULL,
    partition_key      TEXT    NOT NULL,
    content            TEXT    NOT NULL DEFAULT '',
    occurred_on_utc    INTEGER NOT NULL,
    attempt            INTEGER NOT NULL DEFAULT 0,
    status             TEXT    NOT NULL DEFAULT 'open'
                               CHECK (status IN ('open', 'processed', 'dead_letter')),
    processed_on_utc   INTEGER,
    dead_letter_reason TEXT,
    created_at         INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at         INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    CHECK ((status = 'processed') = (processed_on_utc IS NOT NULL)),
    CHECK ((status = 'dead_letter') = (dead_letter_reason IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS outbox_open_idx
    ON outbox (status) WHERE status = 'open';";

// Die wiederholten `unixepoch('subsec')`-Aufrufe (auch in INSERT_MESSAGE_RETURNING) liefern
// **denselben** Zeitpunkt: SQLite berechnet die aktuelle Zeit genau **einmal pro Statement** und
// teilt sie über alle Aufrufe — occurred_on_utc/created_at/updated_at eines Inserts sind damit
// garantiert identisch (empirisch über 10k Inserts bestätigt). Ein gemeinsamer CTE-Wert wäre
// reine Redundanz gegen eine schon zugesicherte Invariante.
const INSERT_MESSAGE: &str = "\
INSERT INTO outbox (id, message_type, partition_key, occurred_on_utc, created_at, updated_at)
VALUES (
    lower(hex(randomblob(16))), ?1, ?2,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
);";

/// Die Outbox-Tabelle als [`TableSpec`] — damit der Schema-Besitzer ([`schema`](crate::schema)) sie
/// in **derselben** Form anlegt wie die Ergebnis-Tabellen der Slices. Er liest davon nur `name` und
/// `ddl`; `insert` steht allein der Vollständigkeit der `TableSpec` halber da — die Outbox schreibt
/// über die [`SqliteUnit`], keinen Store-Preflight.
pub(crate) const OUTBOX_TABLE: TableSpec = TableSpec {
    name: "outbox",
    ddl: OUTBOX_DDL,
    insert: INSERT_MESSAGE,
};

const INSERT_MESSAGE_RETURNING: &str = "\
INSERT INTO outbox (id, message_type, partition_key, occurred_on_utc, created_at, updated_at)
VALUES (
    lower(hex(randomblob(16))), ?1, ?2,
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER),
    CAST(unixepoch('subsec') * 1000 AS INTEGER)
)
RETURNING id;";

const SELECT_OPEN: &str = "\
SELECT id, message_type, partition_key, content, occurred_on_utc, attempt
FROM outbox
WHERE status = 'open'
ORDER BY rowid;";

/// Nur der `partition_key` der ältesten offenen Nachricht **eines** Typs — die schmalste Auskunft für
/// den Vorab-Check in [`run`](crate::run), eine Spalte statt sechs
/// (`.rules/sql/queries-und-performance.md`).
const SELECT_PENDING_KEY: &str = "\
SELECT partition_key
FROM outbox
WHERE status = 'open' AND message_type = ?1
ORDER BY rowid
LIMIT 1;";

/// Ackt **nur eine noch offene** Nachricht (`AND status = 'open'`) und zählt den Versuch mit. Der
/// Zusatz macht den Ack **idempotent**: trifft er keine offene Zeile (bereits verarbeitet / überlappende Zustellung
/// durch einen zweiten Prozess auf derselben Datei), aktualisiert er `0` Zeilen — und der Commit von
/// [`SqliteUnit`] scheitert, statt still Seiteneffekte doppelt zu committen.
const ACK_MESSAGE: &str = "\
UPDATE outbox
SET status           = 'processed',
    attempt          = attempt + 1,
    processed_on_utc = CAST(unixepoch('subsec') * 1000 AS INTEGER),
    updated_at       = CAST(unixepoch('subsec') * 1000 AS INTEGER)
WHERE id = ?1 AND status = 'open';";

/// Mintet eine neue ID **synchron an der DB** — dasselbe Rezept, das jeder slice-spezifische Store
/// beim Ablegen braucht: ein zufälliges 128-Bit-`randomblob`, hex-kodiert.
pub(crate) const MINT_ID: &str = "SELECT lower(hex(randomblob(16)));";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::outbox::OutboxError;
    use crate::pipeline_test_support::TempDbPath;
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
                unit.stage_row(Box::new(move |tx| {
                    tx.execute("INSERT INTO results (note) VALUES (?1);", (note.as_str(),))
                        .map(|_| ())
                }));
                unit.publish(NextCommand);
                if poisoned {
                    unit.stage_row(Box::new(|tx| {
                        tx.execute("INSERT INTO does_not_exist (x) VALUES (1);", ())
                            .map(|_| ())
                    }));
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
            "SELECT status = 'processed' FROM outbox WHERE id = ?1;",
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
                [(message, Delivery::Redelivery { error: OutboxError::Retry { .. }, .. })] if message.id == root
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
                [(message, Delivery::Redelivery { error: OutboxError::Retry { .. }, .. })] if message.id == root
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
            matches!(zweiter_lauf.as_slice(), [(message, Delivery::Processed)] if message.id == root),
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

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn eine_offene_nachricht_ueberlebt_einen_prozess_neustart()
    -> Result<(), Box<dyn std::error::Error>> {
        let db = TempDbPath::new("restart");
        // Der DB-Pfad kommt aus der App-Konfiguration, nicht aus dem Code; geöffnet wird über den
        // produktiven Weg, den Schema-Besitzer.
        let config = AppConfig::new(db.path());

        // Erster Prozess: eine Nachricht einreihen, dann Verbindung schließen (= „Prozess-Ende").
        let root = {
            let outbox = crate::schema::open(&config)?;
            outbox.enqueue(ROOT_TYPE, "root-key")?
        };

        // Neustart: dieselbe Config, dieselbe Datei — die offene Nachricht liegt noch da.
        let outbox = crate::schema::open(&config)?;
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
            matches!(outcomes.as_slice(), [(message, Delivery::Processed)] if message.id == root),
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
            "UPDATE outbox SET status = 'processed', processed_on_utc = 1 WHERE id = ?1;",
            (root.as_str(),),
        )?;

        // Eine zweite Einheit, die dieselbe Nachricht erneut ackt, darf ihre Seiteneffekte **nicht**
        // committen — der idempotente Ack trifft keine offene Zeile.
        let unit = outbox.begin(MessageId(&root));
        unit.stage_row(Box::new(|tx| {
            tx.execute("INSERT INTO results (note) VALUES ('doppelt');", ())
                .map(|_| ())
        }));
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
