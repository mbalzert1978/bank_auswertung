//! `io` — die **Infrastruktur** des modularen Monolithen: hier lebt die Outbox-Mechanik, die
//! die Slices seit [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)
//! **nicht** mehr kennen. `io` hält die Outbox-Seams
//! (`OutboxMessage`/`OutboxCommand`/`OutboxError`/`OutboxHandler`/`OutboxPublisher`/`UnitOfWork`)
//! selbst als **io-privates Modul** [`outbox`] und bindet den Slice↔`io`-Vertrag `application_kernel`
//! (`Handler`/`CommandDecoder`/`Publisher`/…) sowie die Slice-Adapter, die es verdrahtet.
//!
//! **Die generische Consumer-Maschinerie ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):**
//!
//! - [`OutboxConsumer`] ist der **eine** [`outbox::OutboxHandler`] über **jeden**
//!   [`TransactionalHandler`]: er extrahiert die Envelope, ruft den slice-gelieferten
//!   `CommandDecoder` (Shape Y — nur nackte Strings) und dispatcht das Kommando an den
//!   `TransactionalHandler`, der **eine** `UnitOfWork` öffnet, den Slice-Handler **aus ihr** baut
//!   (Store und Publish sind dieselbe committende Unit — eine gestagete Schreibung committet mit
//!   der Nachricht oder gar nicht) und committet. Jeder Handler-Fehler ist **terminal** und
//!   wird über sein **`Display`** gemeldet (nie `Debug`); der Broker sondert die Nachricht dann
//!   sofort aus, einen gescheiterten Commit erst nach der Höchstzahl an Versuchen
//!   ([`sqlite::Delivery`], Issue 0085). Den `OutboxError` der
//!   Nachrichtenverarbeitung baut der Consumer (Retry entsteht ausschließlich io-intern beim
//!   Commit); die Auflösungs-Ebene baut ihr eigenes `Critical` im [`Dispatcher`].
//! - [`PublishingPort`] macht den **letzten Outbox-Hop**: es bindet
//!   `application_kernel::Publisher` an [`outbox::OutboxPublisher`]
//!   (`PublishCommand → OutboxCommand`). **Heute pointer-only:** ein Kommando trägt nur
//!   `message_type` + `partition_key` (Tabelle = Wahrheit, Message = Zeiger), kein `content`.
//!   Ein erster nutzlast-tragender Slice erweitert den Outbox-Vertrag um einen Content-/serde-
//!   Seam — dann ist es kein reiner Zeiger mehr.
//! - [`UnitFactory`] ist die **eine** Unit-Quelle jedes Slice: `begin` öffnet je Nachricht die
//!   Arbeitseinheit, die Ack, Ablage und Publishes gemeinsam committet. Ihr Bound trägt nur, was
//!   **jede** Einheit kann (`UnitOfWork` + `OutboxPublisher`); den slice-eigenen Ablage-Seam
//!   verlangt der jeweilige [`TransactionalHandler`] dort, wo er ihn benutzt (`where F::Unit: …`).
//! - [`Dispatcher`] löst eine Nachricht per `message_type` auf ihren Consumer auf.
//! - [`einlesen`], [`obfuskierung`], [`fenstern`] und [`auswerten`] verdrahten je den
//!   Slice-Handler und -Decoder in einen [`OutboxConsumer`] — dieselbe generische Maschinerie, vier
//!   Slices. [`fenstern`] und [`auswerten`] bringen je **zwei** Consumer mit (`fenstern`: Anstoß und
//!   Fenster-Schritt; `auswerten`: die pro-Fenster- und die fenster-übergreifende Reichweite), die
//!   derselbe [`Dispatcher`] uniform über ihren `message_type` auflöst — die Maschinerie brauchte
//!   dafür keine Erweiterung.
//!
//! **Das reale transaktionale Rückgrat** ([`sqlite`], Issue 0052,
//! [ADR-0014](../../../docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)) ist gebaut: die
//! [`sqlite::SqliteOutbox`] ist Broker **und** Unit-Quelle über `rusqlite` — sie reiht Nachrichten in
//! die SQLite-Outbox-Tabelle, liest die offenen und dispatcht sie über **denselben, unveränderten**
//! [`Dispatcher`]/[`OutboxConsumer`], und die [`sqlite::SqliteUnit`] committet Ack + Ablage + Publishes
//! in **einer** Transaktion (at-least-once, resumierbar). Der **erste slice-spezifische Ergebnis-Store**
//! sitzt seit [Issue 0053](../../../docs/issues/0053-einlesen-store-real-und-end-to-end.md) darauf:
//! [`einlesen::EinlesenSqliteStore`] erfüllt `einlesen`s `StoreSeam` über eine echte `rusqlite`-Tabelle
//! (DB-vergebene ID; Zeilen-`INSERT`, `Ingested`-Publish und Ack in **einer** Transaktion), sodass der
//! `einlesen`-Slice end-to-end **ohne einen Fake** läuft. Sein **Muster** tragen inzwischen **alle
//! vier** Slices: `obfuskierung`, `fenstern` und `auswerten` haben ihren realen Ergebnis-Store je
//! hinter dem eigenen Seam (Issues 0056–0058), und seit
//! [ADR-0015](../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md) steht jeder davon
//! als [`StoreSpec`](sqlite::StoreSpec)-`impl` auf **einem** generischen
//! [`SliceStore`](sqlite::SliceStore). **Alle drei Config-Loader sind real** ([`config`]): Fenster
//! (0054), Seed (0059) und — seit dem Lauf-Anstoß — die Eigene-Konten-Config
//! ([Issue 0002](../../../docs/issues/0002-eigene-konten-config-schema-und-loader.md)), die IBANs
//! liest und **Token** herausgibt.
//!
//! **Und darüber der Lauf** ([`run`],
//! [Issue 0083](../../../docs/issues/0083-lauf-anstoss-binary-reiht-den-ausloeser-ein-und-dreht-die-broker-loop.md)):
//! die Verdrahtung **aller** realen Adapter zu einem Programm — Configs laden, Consumer registrieren,
//! die Auslöser **einreihen** (nie rufen) und die Broker-Schleife drehen, bis keine offene Nachricht
//! mehr bleibt. Damit läuft die Pipeline zum ersten Mal außerhalb eines Tests. Die Bin-Crate `lauf`
//! darüber ist reine Kommandozeile.
//!
//! **Das Kategorie-Register ist real** ([`auswerten::SqliteCategoryRegistry`],
//! [Issue 0009](../../../docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)): eine
//! SQLite-Tabelle Fingerabdruck → Kategorie, die gelernte Korrekturen über jeden Neustart trägt.
//! **Das Kategorisierer-Orakel ist ein Chat** ([Issue 0011](../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md)):
//! im Lauf sammelt der [`auswerten::CollectingCategorizer`] die Anfragen der unbekannten Arten und
//! antwortet leer; [`auswerten::chat_export`] schreibt sie am Ende eines gelungenen Laufs als Datei,
//! [`auswerten::chat_import`] übernimmt die Antwort über das Register. **Noch draußen:** der
//! `report`-Presenter.

mod common;

#[cfg(test)]
mod pipeline_test_support;

pub mod auswerten;
pub mod config;
pub mod einlesen;
pub mod fenstern;
pub mod obfuskierung;
pub mod run;
pub mod schema;
pub mod sqlite;

pub use common::outbox;
pub(crate) use common::run_in_unit;
pub use common::{
    Dispatcher, MessageId, OutboxConsumer, PublishingPort, TransactionError, TransactionalHandler,
    UnitFactory,
};
