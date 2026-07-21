# `io`-Persistenz über rusqlite — die Outbox-Spine bleibt synchron, `sqlx` ist verworfen

> **Status: Akzeptiert** (Entscheidung + Doku; **kein** Adapter-/Store-Code — der reale
> Store ist Issue [0052](../issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md)/
> [0053](../issues/0053-einlesen-store-real-und-end-to-end.md)).

Dieses ADR zieht **eine ripple-schwere `io`-Weiche fest, bevor** irgendein realer
Store-Code entsteht: die reale `io`-Persistenz läuft über **rusqlite** und bleibt
**synchron**. Es stützt sich auf [ADR-0003](0003-rust-als-sprache.md) (ein einziges
lokales Binary **ohne Runtime**, Datenschutz-Bonus) und respektiert die
**synchrone** Outbox-Spine, die [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md),
[ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md) und
[ADR-0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) gebaut haben.
Die Pipeline-Garantien (atomar, at-least-once, wiederaufsetzbar) und die
Datenschutz-Grenze an `obfuskieren` bleiben **unberührt**.

Herkunft: Issue
[0049](../issues/0049-persistenz-stack-rusqlite-synchron-kein-sqlx.md) — die Weiche muss
**vor** der transaktionalen Backbone ([0052](../issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md))
und den Slice-Stores ([0053](../issues/0053-einlesen-store-real-und-end-to-end.md))
festgezogen sein; die persistenzfreien Schnitte
([0050](../issues/0050-money-calendar-decode-seams-real.md),
[0051](../issues/0051-einlesen-csv-camt-v8-quelle-real-hinter-source-seam.md),
[0054](../issues/0054-config-loader-toml-fenster-config.md)) hängen **nicht** daran.

## Problem

Die gesamte Outbox-Spine ist **bewusst synchron** gebaut: `Handler::handle`,
`TransactionalHandler::handle`, `UnitOfWork::commit` und `OutboxHandler::handle` sind
allesamt **synchrone `fn`** — es gibt **keine** async-Runtime im Prozess
([ADR-0003](0003-rust-als-sprache.md): ein einziges lokales Binary ohne Runtime, ein
bewusster Datenschutz-Bonus). Der reale `io`-Store muss hinter genau diese sync-Traits
fallen.

`sqlx` ist **async-first**: seine Query-/Transaktions-API ist durchweg `async fn`. Es in
die synchrone Spine hereinzuziehen ließe nur zwei Wege — beide schlecht:

1. **Die komplette Kette auf async umbauen** — `Handler`/`TransactionalHandler`/
   `UnitOfWork`/`OutboxHandler` würden `async fn`, und der `async`-Ripple liefe durch
   `application_kernel` (den einzigen Slice↔`io`-Vertrag) und **jeden** Slice-Adapter,
   der ihn bindet. Der Ripple träfe damit Crates, die von I/O gar nichts wissen sollen,
   und zöge eine async-Runtime in ein Programm, das [ADR-0003](0003-rust-als-sprache.md)
   bewusst **ohne** Runtime hält.
2. **Auf einer Runtime `block_on`** — man behielte die sync-Signaturen und blockierte in
   jedem Store-Aufruf auf einer versteckten Runtime (`futures::executor::block_on` o. ä.).
   Das ist ein **Hack**: eine async-Maschinerie inklusive Runtime nur, um sie sofort
   wieder synchron zu blockieren — Komplexität und Laufzeitkosten ohne Gegenwert für ein
   lokales Single-User-Analysetool ohne Nebenläufigkeitsbedarf.

## Entscheidung — `io`-Persistenz = rusqlite, synchron

- **Die reale `io`-Persistenz läuft über [rusqlite](https://crates.io/crates/rusqlite)**
  (SQLite-Binding mit **synchroner** API). rusqlite fällt hinter die bestehenden
  sync-Traits, **ohne eine einzige Signatur zu ändern**: `Connection`/`Transaction` sind
  synchron, `Transaction` rollt bei `Drop` zurück (genau die Semantik, die
  `UnitOfWork`/`OutboxPublisher` heute vom Fake erwarten).
- **Die Outbox-Spine bleibt synchron.** Kein `async fn`, keine Runtime, kein Ripple durch
  `application_kernel` und die Slice-Adapter. Der Broker-Loop, der generische
  `TransactionalHandler` und die `UnitOfWork` behalten ihre sync-Signaturen; der reale
  Store fügt sich als sync-Adapter hinter die vorhandenen Seams ein.
- **`sqlx` ist verworfen.** Kein async-first-Treiber in einem bewusst runtime-freien,
  synchronen, lokalen Binary.

## Begründung

- **Synchron passt zu synchron.** rusqlites synchrone API fällt hinter die bestehenden
  sync-Traits ohne eine einzige Signaturänderung — kein `async`-Ripple, keine Runtime
  gegen [ADR-0003](0003-rust-als-sprache.md) (der abgewogene Vergleich mit den beiden
  `sqlx`-Wegen steht oben unter **Problem**).
- **Runtime-los bleibt echt.** Ohne async-Treiber gibt es nichts zu blockieren; die
  Runtime-los-Garantie aus [ADR-0003](0003-rust-als-sprache.md) bleibt echt statt
  versteckt-blockierend.
- **Passt zum Einsatzbild.** Bank-Auswertung ist ein **lokales**, single-user
  Analysewerkzeug ohne Server-Nebenläufigkeit. Async-I/O kauft hier nichts; SQLite-Zugriff
  ist lokal und synchron ausreichend.
- **`Transaction`-Rollback-bei-Drop** von rusqlite deckt die at-least-once-/
  wiederaufsetzbar-Garantie der `UnitOfWork` (committet atomar oder gar nicht) ohne
  zusätzliche Maschinerie.

## Konsequenzen

- Der reale `io`-Store (Issues
  [0052](../issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md)/
  [0053](../issues/0053-einlesen-store-real-und-end-to-end.md)) wird mit **rusqlite** als
  externe Crate gebaut — sie lebt, wie alle Fremd-Crates, **ausschließlich im `io`-Crate**
  ([ADR-0007](0007-modularer-monolith-feature-slices.md), Dependency-Richtung; die
  `architecture`-Prüfung und der PreToolUse-Hook halten das durch).
- **Keine async-Runtime** wird ins Binary gezogen; `application_kernel` und die
  Slice-Adapter bleiben von `async` unberührt.
- **Reine Doku-/Entscheidungs-Änderung — kein Code-Verhalten ändert sich.** Die heutigen
  In-Memory-Fakes und alle sechs Gates bleiben grün; dieses ADR ist der **kanonische Ort
  der Entscheidung**, keine Code-Änderung selbst.

## Verworfen, mit Grund

- **`sqlx` + async-Umbau der Spine** — breiter `async`-Ripple durch `application_kernel`
  und alle Slice-Adapter, plus eine Runtime gegen [ADR-0003](0003-rust-als-sprache.md).
- **`sqlx` + `block_on`** — async-Maschinerie samt Runtime, nur um sie synchron zu
  blockieren; ein Hack ohne Gegenwert für ein lokales, nebenläufigkeitsfreies Tool.

## Bewusst vertagt

Der reale Store selbst — SQLite-Outbox-Broker/-Stores, `UnitOfWork` mit **einer**
rusqlite-`Transaction` pro Nachricht, die Tag-Stamp-Persistenz
([ADR-0013](0013-zeitfenster-collector-paginierender-analyse-kopf.md), Entscheidung C) —
ist Issue [0052](../issues/0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md)/
[0053](../issues/0053-einlesen-store-real-und-end-to-end.md), nicht dieses ADR.
