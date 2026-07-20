---
id: "0052"
title: "SQLite-Outbox-Broker + UnitOfWork real (rusqlite)"
status: open
---

# SQLite-Outbox-Broker + UnitOfWork real (rusqlite)

**Typ:** AFK

## What to build

Das echte transaktionale **Rückgrat** in `io`, als Ersatz der In-Memory-Doubles der io-privaten
Outbox-Seams (`UnitOfWork`, `OutboxPublisher`, Broker/Consumer-Trigger) — der Teil, den PROGRESS.md
bislang bewusst „vertagt, kein Issue" gehalten hat.

- **`UnitOfWork` (rusqlite)** umspannt **eine** Transaktion: Ack, Ergebnis-Ablage und Publishes
  committen atomar; ein Fehlschlag rollt **alles** zurück.
- **Outbox-Tabelle + Broker:** die publizierte Nachricht landet in der SQLite-Outbox-Tabelle; der
  Broker liest offene Nachrichten und dispatcht sie über den **bestehenden** generischen
  `Dispatcher`/`OutboxConsumer` (unverändert). At-least-once, resumierbar.
- Der Slice bleibt unangetastet: Consumer-Rolle und Unit-of-Work liegen **io-privat**
  ([ADR-0011](../adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)), die Seams bleiben io-privat.

**Nur Broker + UoW als Fundament** — noch **kein** slice-spezifischer Ergebnis-Store; der erste Store
kommt in [0053](0053-einlesen-store-real-und-end-to-end.md) und wird pro Slice repliziert. Reframt den
Persistenz-Teil des überholten [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md) auf den
heutigen io-privaten Seam-Stand (0009 wird dadurch nicht neu geschnitten, nur abgelöst).

## Acceptance criteria

- [ ] Eine rusqlite-`UnitOfWork` committet Ack + Ablage + Publish in **einer** Transaktion; ein
      Fehlschlag rollt alles zurück (Test).
- [ ] Die SQLite-Outbox-Tabelle nimmt Nachrichten auf; der Broker dispatcht offene Nachrichten über den
      **bestehenden** `Dispatcher` — keine neue Consumer-Mechanik.
- [ ] At-least-once + resumierbar: eine nach Commit, vor Ack „abgestürzte" Nachricht wird erneut
      zugestellt (Test).
- [ ] Die rusqlite-Dep liegt **nur in `io`**; die Outbox-Seams bleiben io-privat; `cargo test -p
      architecture` bleibt grün.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`,
      `cargo test -p architecture` und `cargo doc --no-deps --workspace` sind grün.

## Blocked by

- Blocked by [0049](0049-persistenz-stack-rusqlite-synchron-kein-sqlx.md) — der Persistenz-Stack muss
  auf rusqlite/synchron festgezogen sein, bevor die transaktionale Backbone entsteht.
