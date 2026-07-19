---
id: "0039"
title: io — fenstern end-to-end über die generische OutboxConsumer
status: blocked
---

# io — fenstern end-to-end über die generische OutboxConsumer

**Typ:** AFK

## What to build

Den `fenstern`-Slice in `io` verdrahten — genau wie `einlesen` und `obfuskierung` schon verdrahtet
sind (vgl. [0020](0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md) /
[0021](0021-obfuskieren-end-to-end-ber-die-generische-outboxconsumer-in-io.md)):

Beide Kommandotypen des Slice werden über den **Dispatcher** uniform aufgelöst; **Consumer-Rolle,
Fehler-Taktik und Unit-of-Work bleiben in `io`** (ADR-0011) — der Slice-Adapter kennt keine
Outbox-Mechanik. Die Seams des Adapters werden mit **In-Memory-Implementierungen** gefüllt; echtes
`io` (SQLite, `sqlx`, Tag-Stamp-Persistenz, Broker-Trigger) bleibt **out of scope**.

Ein **End-to-End-Test** treibt den ganzen Lauf: ein `StartAnalysis` hinein — mehrere Seiten mit je
einem abgelegten `Auswertungsfenster` und `FensterGeschlossen` heraus — am Ende genau ein
`HistorieVollständig`. Ohne Datei, ohne Netz, ohne echte Datenbank.

## Acceptance criteria

- [ ] Ein `StartAnalysis` auf der Outbox durchläuft die **volle** Kette bis `HistorieVollständig`,
      mit einem `FensterGeschlossen` je Seite.
- [ ] **Beide** Kommandotypen werden vom Dispatcher uniform aufgelöst; jeder Handler bleibt
      einzweckig.
- [ ] Die Unit-of-Work umspannt Consume-Ack, Ablage und Publishes; ein Fehlschlag rollt **alles**
      zurück, sodass keine halb-fortgesetzte Kette entsteht.
- [ ] Der Slice-Adapter bindet **keine** Outbox-Consumer-Mechanik — der `architecture`-Test bleibt
      grün.
- [ ] Der Test läuft ohne Datei-, Netz- oder echten Datenbankzugriff.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

- Blocked by [0037](0037-fenstern-selbst-fortsetzung-mehr-seiten-paging.md) — der Mehr-Seiten-Lauf
  ist genau das, was der End-to-End-Test treibt.
- Blocked by [0038](0038-fenstern-startanalysis-kickoff-und-leere-datenbank.md) — der Test startet
  am `StartAnalysis`.
