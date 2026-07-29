//! Der **generische Slice-Ergebnis-Store** über dem SQLite-Rückgrat
//! ([ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md),
//! [Issue 0074](../../../../docs/issues/0074-storespec-abstraktion-einf-hren-und-einlesen-migrieren.md)).
//!
//! Die realen Slice-Ergebnis-Stores wiederholten dasselbe 6-teilige Skelett über
//! [`SqliteUnit`]: Store-Struct · `new` · `UnitFactory` · `Unit`-Newtype · Seam-Body-„Tanz" · `Row`.
//! Slice-spezifisch ist einzig **seine [`TableSpec`]-Liste + Row-Flatten** — genau das, und nur das,
//! verlangt [`StoreSpec`]. Das Skelett trägt [`SliceStore`]/[`SliceUnit`] **einmal**; ein Slice
//! liefert nur noch sein `StoreSpec`-`impl` und eine dünne Seam-`impl`, die
//! [`SliceUnit::store_dto`] aufruft. Seit dem `auswerten`-Capstone
//! ([Issue 0077](../../../../docs/issues/0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md))
//! gilt das auch für die Unit-Quelle: die vier strukturgleichen `UnitFactory`-Traits der Slices
//! (Unterschied: einzig der Seam-Bound auf `type Unit`) sind zu **einem** [`UnitFactory`]
//! kollabiert, das hier **einmal** generisch erfüllt wird.
//!
//! **Der Seam-Vertrag Richtung Slice bleibt unberührt** ([ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):
//! diese Abstraktion gehört **nicht** zum Slice↔`io`-Vertrag. Sie ist nur deshalb `pub`, weil die
//! öffentlichen Store-Aliase der Slices sie in ihrer Signatur tragen (ein privates Trait als Bound
//! einer öffentlichen `impl` wäre `private_bounds`); implementiert wird sie ausschließlich `io`-intern,
//! und die tragende Methode ([`store_dto`](SliceUnit::store_dto)) bleibt crate-privat. Jeder Store
//! zeigt weiterhin explizit, **welchen** Seam er erfüllt — versteckt wird nur die Mechanik, nicht
//! die Slice-Fläche.
//!
//! **Ablauf der Ablage-Primitive** ([ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)):
//! fehlbares [`flatten`](StoreSpec::flatten) → Preflight **aller** [`TableSpec::insert`] → ID an der
//! DB minten → [`stage`](StoreSpec::stage) baut die id-**tragende** Row in einem Zug mit der frisch
//! geminteten ID. Dass sie DB-vergeben ist, trägt seit
//! [Issue 0082](../../../../docs/issues/0082-gemintete-zeilen-id-als-mintedid-newtype-durch-den-slice-store-seam-fuehren.md)
//! der Typ [`MintedId`] statt einer Konvention; zurückgegeben wird sie **stets** als `String`, und ob
//! ein Slice sie als Zeiger publiziert, ist Slice-Sache (das Pipeline-Ende verwirft sie per `map(drop)`).

use std::marker::PhantomData;

use rusqlite::types::{ToSql, ToSqlOutput};
use rusqlite::{Connection, Transaction};

use crate::outbox::{OutboxCommand, OutboxPublisher, UnitOfWork};
use crate::sqlite::{
    AUDIT_COLUMNS, MINT_ID, SqliteCommitError, SqliteOutbox, SqliteUnit, ensure_columns,
};
use crate::{MessageId, UnitFactory};

/// Eine SQL-fertige, **besitzende** Ablage-Schreibung: sie reist in die gestagete Closure und wird
/// erst am Commit-Rand in die committende Transaktion geschrieben.
pub type StagedRow = Box<dyn FnOnce(&Transaction<'_>) -> rusqlite::Result<()> + 'static>;

/// Eine Tabelle des Slice mit **ihrem** DDL und **ihrem** Ablage-Statement — beisammen statt in
/// parallelen Listen. Eine Tabelle ohne Audit-Spalten-Nachrüstung oder ohne Preflight ist damit gar
/// nicht erst deklarierbar: [`SliceStore::new`] und der Preflight laufen über **dieselbe** Liste.
///
/// Die SQL bleibt pro Slice als `&'static str`-Literal authored — der generische Store fasst **keine**
/// DDL an und leitet nichts aus SQL ab (kein Parsen, keine Magie): er liest nur, was der Slice
/// deklariert hat.
pub struct TableSpec {
    /// Der Tabellenname — Ziel der Audit-Spalten-Nachrüstung (`ensure_columns`). Stets ein
    /// Code-Literal, nie Fremd-Eingabe.
    pub name: &'static str,
    /// Das `CREATE TABLE IF NOT EXISTS …` dieser Tabelle samt ihrer `CREATE INDEX IF NOT EXISTS`.
    pub ddl: &'static str,
    /// Das Ablage-`INSERT` dieser Tabelle — der Preflight probt es, sodass eine fehlende Tabelle
    /// **in der Ablage** terminal failt statt erst als Commit-Retry.
    pub insert: &'static str,
}

/// Die **an der DB gemintete** Zeilen-ID — der Beweis, dass diese ID aus dem Mint-Zweig stammt und
/// nicht von Hand erfunden wurde ([Issue 0082](../../../../docs/issues/0082-gemintete-zeilen-id-als-mintedid-newtype-durch-den-slice-store-seam-fuehren.md),
/// [ADR-0005](../../../../docs/adr/0005-domaenenmodell-value-objects-und-summentypen.md)).
///
/// Ihr Feld ist privat und sie trägt **keinen** Konstruktor: gebaut wird sie einzig in
/// `preflight_and_mint`, dem einen Ort, an dem `MINT_ID` läuft. Eine leere oder frei erfundene ID —
/// die unter einem Schein-Primärschlüssel schriebe und als publizierter Zeiger stromabwärts reiste —
/// ist damit außerhalb dieses Moduls ein **Compile-Fehler**, keine Review-Frage.
///
/// Geschrieben wird sie über ihr [`ToSql`], nicht über einen Getter: die Stores reichen sie
/// unverändert in ihre `params!` durch, ohne sie je in einen `String` zurückzuverwandeln.
/// **Der Seam-Vertrag Richtung Slice bleibt unberührt** ([ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):
/// die Ablage-Primitive gibt weiterhin die `String`-ID zurück, die der Slice als Zeiger publiziert.
pub struct MintedId(String);

impl MintedId {
    /// Die gemintete ID als Zeichenkette — für die Rückgabe an den Seam, **nicht** als Umweg zu einem
    /// neuen `MintedId` (der Weg zurück fehlt bewusst).
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl ToSql for MintedId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

/// Die **ganze Persistenz eines Slice** in vier Bausteinen: seine Tabellen (je mit DDL und `INSERT`,
/// siehe [`TableSpec`]), die fehlbare Umdeutung der DTO, das Bauen der id-tragenden Row und der
/// redigierte Fehler-Konstruktor. Alles Übrige — Struct, `new`, Unit-Newtype, Preflight, Minten,
/// Outbox-Passthrough — trägt [`SliceStore`]/[`SliceUnit`] generisch.
///
/// **Rest-Drift, bewusst** ([ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md)):
/// [`stage`](StoreSpec::stage) fährt die Statements, die [`TableSpec::insert`] deklariert — dass es
/// dieselben sind, hält der Slice von Hand (beide zeigen auf dasselbe `&'static str`-Literal, das
/// neben seiner Tabelle steht). Ein in `stage` gefahrenes, nirgends deklariertes Statement entginge
/// dem Preflight. Das Netz ist der pro Slice handgeschriebene Store-Test.
pub trait StoreSpec {
    /// Die Adapter-DTO, die der Seam hereinreicht.
    type Dto;
    /// Die **vollständige, besitzende und id-lose** Speicher-Form der DTO, wie [`flatten`](StoreSpec::flatten)
    /// sie liefert: alles Fehlbare (u64→i64-Beträge, Positionsvorläufe) ist hier schon entschieden,
    /// die ID fehlt noch. Sie ist **nicht** schreibbar — schreibbar wird erst die Row, die
    /// [`stage`](StoreSpec::stage) aus ihr **mit** der ID baut.
    type Prepared;
    /// Der adaptereigene, **redigierte** Store-Fehler des Slice (`StoreUnavailable`).
    type Error;

    /// Alle Tabellen des Slice, je mit DDL und Ablage-`INSERT` ([`TableSpec`]) — `new` legt jede an
    /// und rüstet ihre Audit-Spalten nach, der Preflight probt jedes ihrer Statements.
    fn tables() -> &'static [TableSpec];

    /// Deutet die DTO fehlbar in ihre besitzende Speicher-Form um — der **einzige** DTO-Kontakt des
    /// Spec. Läuft **vor** dem Minten, damit `stage` die Row in einem Zug mit der ID bauen kann.
    fn flatten(dto: &Self::Dto) -> Result<Self::Prepared, Self::Error>;

    /// Baut die SQL-fertige, **id-tragende** Row in einem Zug mit der frisch geminteten `id` und
    /// liefert **eine** Schreibung, die Kopf- und alle Detail-Inserts fährt. Die ID ist stets erster
    /// Konstruktor-Parameter der Row; dass sie die **eben gemintete** ist, trägt der Typ
    /// [`MintedId`] — eine von Hand gebaute Schein-ID ist hier gar nicht erst übergebbar.
    fn stage(prepared: Self::Prepared, id: MintedId) -> StagedRow;

    /// Der **redigierte** Konstruktor des Slice-Fehlers aus einer `rusqlite`-Diagnose: reine
    /// Zugriffs-Mechanik, nie ein DTO-Feld- oder Zellwert.
    fn unavailable(error: rusqlite::Error) -> Self::Error;
}

/// Der generische Slice-Ergebnis-Store: hält das SQLite-Rückgrat und stellt beim Aufbau die
/// Tabellen des Slice sicher. Ein Slice bindet ihn als Typ-Alias unter seinem sprechenden Namen und
/// erfüllt damit seine `UnitFactory`. `Clone` teilt dasselbe Rückgrat (`Rc` im Innern).
pub struct SliceStore<S> {
    outbox: SqliteOutbox,
    /// `fn() -> S` statt `S`: der Store **besitzt** kein `S` (der Spec ist ein reiner Marker), also
    /// soll er auch dessen Auto-Traits und Drop-Check nicht erben.
    spec: PhantomData<fn() -> S>,
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
    /// Anlegen und Nachrüsten laufen über **eine** Liste, also je Tabelle beides oder keines.
    pub fn new(outbox: SqliteOutbox) -> rusqlite::Result<Self> {
        outbox.with_connection(|conn| {
            S::tables().iter().try_for_each(|table| {
                conn.execute_batch(table.ddl)?;
                ensure_columns(conn, table.name, AUDIT_COLUMNS)
            })
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

/// **Die** [`UnitFactory`] jedes migrierten Slice — **ein** generisches `impl` statt vier
/// handgeschriebener Durchgriffe ([ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md),
/// [Issue 0077](../../../../docs/issues/0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md)):
/// welchen Ablage-Seam die geöffnete Einheit zusätzlich erfüllt, verlangt der jeweilige
/// [`TransactionalHandler`](crate::TransactionalHandler) dort, wo er ihn benutzt.
impl<S: StoreSpec> UnitFactory for SliceStore<S> {
    type Unit = SliceUnit<S>;

    fn begin(&self, ack: MessageId<'_>) -> SliceUnit<S> {
        SliceUnit(self.outbox.begin(ack), PhantomData)
    }
}

/// Eine generische Slice-Arbeitseinheit: der gestagete Ergebnis-Speicher **und** die Outbox in
/// einem, atomar committbar — ein Newtype über der [`SqliteUnit`], der ihr über [`StoreSpec`] die
/// slice-spezifische Ablage gibt und Publish/Commit durchreicht.
///
/// `fn() -> S` statt `S` aus demselben Grund wie beim [`SliceStore`]: die Einheit **besitzt** kein
/// `S` und soll dessen Auto-Traits und Drop-Check nicht erben.
pub struct SliceUnit<S>(SqliteUnit, PhantomData<fn() -> S>);

impl<S: StoreSpec> SliceUnit<S> {
    /// Die generische **Ablage-Primitive**, auf die jede Seam-`impl` schrumpft: fehlbares
    /// [`flatten`](StoreSpec::flatten) → Preflight **aller** [`TableSpec::insert`] → ID an der DB
    /// minten → [`stage`](StoreSpec::stage) mit dieser ID, gestaget in die committende Transaktion.
    /// Gibt **stets** die DB-vergebene ID zurück; ein unerreichbarer Store (Schema fehlt /
    /// DB-Fehler) failt **hier** terminal als redigierter `StoreSpec::Error`.
    pub(crate) fn store_dto(&self, dto: &S::Dto) -> Result<String, S::Error> {
        let prepared = S::flatten(dto)?;
        let minted = self
            .0
            .with_connection(Self::preflight_and_mint)
            .map_err(S::unavailable)?;
        let id = minted.as_str().to_owned();
        self.0.stage_row(S::stage(prepared, minted));
        Ok(id)
    }

    /// Probt **jedes** Ablage-Statement auf Präparierbarkeit (Store erreichbar?) und mintet dann die
    /// neue Zeilen-ID synchron an der DB. Geprobt, nicht ausgeführt. **Der einzige Ort, an dem eine
    /// [`MintedId`] entsteht** — sie kann darum nichts anderes sein als das, was `MINT_ID` geliefert hat.
    fn preflight_and_mint(conn: &Connection) -> rusqlite::Result<MintedId> {
        S::tables()
            .iter()
            .try_for_each(|table| conn.prepare(table.insert).map(drop))?;
        conn.query_row(MINT_ID, [], |row| row.get(0)).map(MintedId)
    }
}

// Der Outbox-Passthrough **einmal** generisch — das frühere, pro Store aufgerufene
// `sqlite_backed_unit!`-Makro ist mit dem letzten migrierten Store entfallen (Issue 0077). Die
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
