//! Der **generische Slice-Ergebnis-Store** über dem SQLite-Rückgrat
//! ([ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md),
//! [Issue 0074](../../../../docs/issues/0074-storespec-abstraktion-einf-hren-und-einlesen-migrieren.md)).
//!
//! Die realen Slice-Ergebnis-Stores wiederholten dasselbe 6-teilige Skelett über
//! [`SqliteUnit`]: Store-Struct · `new` · `UnitFactory` · `Unit`-Newtype · Seam-Body-„Tanz" · `Row`.
//! Slice-spezifisch ist einzig **Schema + Insert(s) + Row-Flatten** — genau das, und nur das,
//! verlangt [`StoreSpec`]. Das Skelett trägt [`SliceStore`]/[`SliceUnit`] **einmal**; ein Slice
//! liefert sein `StoreSpec`-`impl` und eine dünne Seam-`impl`, die [`SliceUnit::store_dto`] aufruft.
//!
//! **Der Seam-Vertrag Richtung Slice bleibt unberührt** ([ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):
//! diese Abstraktion ist rein `io`-intern. Jeder Store zeigt weiterhin explizit, **welchen** Seam er
//! erfüllt — versteckt wird nur die Mechanik, nicht die Slice-Fläche.
//!
//! **Ablauf der Ablage-Primitive** ([ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)):
//! fehlbares [`flatten`](StoreSpec::flatten) → Preflight **aller** [`inserts`](StoreSpec::inserts) →
//! ID an der DB minten → [`stage`](StoreSpec::stage) baut die id-**tragende** Row in einem Zug mit der
//! frisch geminteten ID. Die ID ist **stets** DB-vergeben und wird **stets** zurückgegeben; ob ein
//! Slice sie als Zeiger publiziert, ist Slice-Sache (das Pipeline-Ende verwirft sie per `map(drop)`).

use std::marker::PhantomData;

use rusqlite::{Connection, Transaction};

use crate::MessageId;
use crate::outbox::{OutboxCommand, OutboxPublisher, UnitOfWork};
use crate::sqlite::{
    AUDIT_COLUMNS, MINT_ID, SqliteCommitError, SqliteOutbox, SqliteUnit, ensure_columns,
};

/// Eine SQL-fertige, **besitzende** Ablage-Schreibung: sie reist in die gestagete Closure und wird
/// erst am Commit-Rand in die committende Transaktion geschrieben.
pub type StagedRow = Box<dyn FnOnce(&Transaction<'_>) -> rusqlite::Result<()> + 'static>;

/// Die **ganze Persistenz eines Slice** in fünf Bausteinen: Tabellen, Schema, Statements und die
/// beiden Hälften der Row-Umdeutung. Alles Übrige — Struct, `new`, Unit-Newtype, Preflight, Minten,
/// Outbox-Passthrough — trägt [`SliceStore`]/[`SliceUnit`] generisch.
///
/// **Intra-`impl`-Konsistenz ist Handarbeit** (bewusst, [ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md)):
/// [`tables`](StoreSpec::tables) und [`schema`](StoreSpec::schema) sind zwei Sichten desselben
/// Schemas, ebenso [`inserts`](StoreSpec::inserts) und [`stage`](StoreSpec::stage). Eine in
/// `schema` angelegte, in `tables` vergessene Tabelle bekäme lautlos keine Audit-Spalten-Nachrüstung;
/// ein in `stage` gefahrenes, in `inserts` fehlendes Statement entginge dem Preflight. Das Netz ist
/// der pro Slice handgeschriebene Store-Test.
pub trait StoreSpec {
    /// Die Adapter-DTO, die der Seam hereinreicht.
    type Dto;
    /// Die fehlbare, **id-lose** Vor-Umdeutung (u64→i64-Beträge, Positionsvorläufe) — sie läuft
    /// **vor** dem Minten, damit `stage` die Row in einem Zug mit der ID bauen kann.
    type Prepared;
    /// Der adaptereigene, **redigierte** Store-Fehler des Slice (`StoreUnavailable`).
    type Error;

    /// Alle Tabellen des Slice — `new` rüstet auf jeder die Audit-Spalten nach.
    fn tables() -> &'static [&'static str];

    /// Das `CREATE TABLE …`-Schema **aller** Tabellen des Slice (`IF NOT EXISTS`).
    fn schema() -> &'static str;

    /// **Alle** Ablage-Statements (Kopf + Detail) — der Preflight probt jedes, sodass eine fehlende
    /// Tabelle **hier** terminal failt statt erst als Commit-Retry.
    fn inserts() -> &'static [&'static str];

    /// Die fehlbare, id-lose Vor-Umdeutung der DTO. Läuft **vor** dem Minten.
    fn flatten(dto: &Self::Dto) -> Result<Self::Prepared, Self::Error>;

    /// Baut die SQL-fertige, **id-tragende** Row in einem Zug mit der frisch geminteten `id` und
    /// liefert **eine** Schreibung, die Kopf- und alle Detail-Inserts fährt. Eine Row ohne ihre ID
    /// ist so gar nicht erst konstruierbar.
    fn stage(dto: &Self::Dto, prepared: Self::Prepared, id: String) -> StagedRow;

    /// Der **redigierte** Konstruktor des Slice-Fehlers aus einer `rusqlite`-Diagnose: reine
    /// Zugriffs-Mechanik, nie ein DTO-Feld- oder Zellwert.
    fn unavailable(error: rusqlite::Error) -> Self::Error;
}

/// Der generische Slice-Ergebnis-Store: hält das SQLite-Rückgrat und stellt beim Aufbau die
/// Tabellen des Slice sicher. Ein Slice bindet ihn als Typ-Alias unter seinem sprechenden Namen und
/// erfüllt damit seine `UnitFactory`. `Clone` teilt dasselbe Rückgrat (`Rc` im Innern).
pub struct SliceStore<S> {
    outbox: SqliteOutbox,
    spec: PhantomData<S>,
}

// Von Hand statt `derive`: die abgeleitete Implementierung verlangte unnötig `S: Clone`, obwohl der
// Spec-Marker gar nichts mitträgt.
impl<S> Clone for SliceStore<S> {
    fn clone(&self) -> Self {
        Self {
            outbox: self.outbox.clone(),
            spec: PhantomData,
        }
    }
}

impl<S: StoreSpec> SliceStore<S> {
    /// Baut den Store über dem gegebenen Rückgrat und legt die Tabellen des Slice an
    /// (`IF NOT EXISTS` — ein bestehender Bestand bleibt erhalten). `ensure_columns` rüstet die
    /// Audit-Spalten auf einer **alten** Tabelle nach, da `CREATE TABLE IF NOT EXISTS` das nicht tut.
    pub fn new(outbox: SqliteOutbox) -> rusqlite::Result<Self> {
        outbox.with_connection(|conn| {
            conn.execute_batch(S::schema())?;
            S::tables()
                .iter()
                .try_for_each(|table| ensure_columns(conn, table, AUDIT_COLUMNS))
        })?;
        Ok(Self {
            outbox,
            spec: PhantomData,
        })
    }

    /// Baut den Store **ohne** sein Schema anzulegen — nur für Tests, die einen *unerreichbaren*
    /// Store brauchen (die Ablage muss dann synchron und terminal failen).
    #[cfg(test)]
    pub(crate) const fn without_schema(outbox: SqliteOutbox) -> Self {
        Self {
            outbox,
            spec: PhantomData,
        }
    }

    /// Öffnet die Arbeitseinheit für die Nachricht `ack` — der Rumpf jeder slice-eigenen
    /// `UnitFactory`.
    pub(crate) fn begin(&self, ack: MessageId<'_>) -> SliceUnit<S> {
        SliceUnit(self.outbox.begin(ack), PhantomData)
    }

    /// Reicht die geteilte Verbindung an `f` — der Lese-Zugriff auf den abgelegten Bestand. Heute
    /// **nur für Tests**: der reale Lesepfad eines Slice liegt hinter dessen eigenem Lese-Seam, nicht
    /// hinter dem Store.
    #[cfg(test)]
    pub(crate) fn with_connection<R>(
        &self,
        f: impl FnOnce(&Connection) -> rusqlite::Result<R>,
    ) -> rusqlite::Result<R> {
        self.outbox.with_connection(f)
    }
}

/// Eine generische Slice-Arbeitseinheit: der gestagete Ergebnis-Speicher **und** die Outbox in
/// einem, atomar committbar — ein Newtype über der [`SqliteUnit`], der ihr über [`StoreSpec`] die
/// slice-spezifische Ablage gibt und Publish/Commit durchreicht.
pub struct SliceUnit<S>(SqliteUnit, PhantomData<S>);

impl<S: StoreSpec> SliceUnit<S> {
    /// Die generische **Ablage-Primitive**, auf die jede Seam-`impl` schrumpft: fehlbares
    /// [`flatten`](StoreSpec::flatten) → Preflight **aller** [`inserts`](StoreSpec::inserts) → ID an
    /// der DB minten → [`stage`](StoreSpec::stage) mit dieser ID, gestaget in die committende
    /// Transaktion. Gibt **stets** die DB-vergebene ID zurück; ein unerreichbarer Store (Schema
    /// fehlt / DB-Fehler) failt **hier** terminal als redigierter `StoreSpec::Error`.
    pub(crate) fn store_dto(&self, dto: &S::Dto) -> Result<String, S::Error> {
        let prepared = S::flatten(dto)?;
        let id: String = self
            .0
            .with_connection(Self::preflight_and_mint)
            .map_err(S::unavailable)?;
        self.0.stage_row(S::stage(dto, prepared, id.clone()));
        Ok(id)
    }

    /// Probt **jedes** Ablage-Statement auf Präparierbarkeit (Store erreichbar?) und mintet dann die
    /// neue Zeilen-ID synchron an der DB. Geprobt, nicht ausgeführt.
    fn preflight_and_mint(conn: &Connection) -> rusqlite::Result<String> {
        S::inserts()
            .iter()
            .try_for_each(|statement| conn.prepare(statement).map(drop))?;
        conn.query_row(MINT_ID, [], |row| row.get(0))
    }
}

// Der Outbox-Passthrough **einmal** generisch, statt pro Store per `sqlite_backed_unit!`-Makro: die
// `SqliteUnit` implementiert beides bereits, der Newtype reicht wortgleich durch.
impl<S> OutboxPublisher for SliceUnit<S> {
    fn publish<T: OutboxCommand>(&self, command: T) {
        self.0.publish(command);
    }
}

impl<S> UnitOfWork for SliceUnit<S> {
    type Error = SqliteCommitError;

    fn commit(self) -> Result<(), SqliteCommitError> {
        self.0.commit()
    }
}
