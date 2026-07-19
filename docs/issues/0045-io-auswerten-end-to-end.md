---
id: "0045"
title: io — auswerten end-to-end über die generische OutboxConsumer
status: blocked
---

# io — auswerten end-to-end über die generische OutboxConsumer

**Typ:** AFK

## What to build

Den `auswerten`-Slice in `io` verdrahten und die **ganze Pipeline gegen Fakes** schließen:

**Beide** Kommandotypen (`FensterGeschlossen`, `HistorieVollständig`) werden über den **Dispatcher**
uniform aufgelöst; **Consumer-Rolle und Unit-of-Work bleiben in `io`** (ADR-0011), die Seams des
Adapters bekommen In-Memory-Implementierungen. Echtes `io` (SQLite/`sqlx`, Tag-Stamp-Persistenz,
Broker-Trigger) bleibt weiterhin **out of scope**.

Ein **End-to-End-Test** treibt die Kette über **beide Slices**: ein `StartAnalysis` hinein — `fenstern`
paginiert und materialisiert die Fenster, `auswerten` rechnet je Fenster die periodische Kennzahl und
am Schluss, auf `HistorieVollständig`, die fenster-übergreifende Query. Damit ist zum ersten Mal die
**volle Pipeline** `einlesen → obfuskieren → fenstern → auswerten` gegen Fakes lauffähig.

## Acceptance criteria

- [ ] Ein `StartAnalysis` durchläuft `fenstern` **und** `auswerten` bis zu **beiden** Ergebnissen:
      den Pro-Fenster-Kennzahlen und dem fenster-übergreifenden Query-Ergebnis.
- [ ] Beide `auswerten`-Kommandotypen werden uniform aufgelöst; **jeder Handler bleibt einzweckig**.
- [ ] Die Unit-of-Work umspannt je Schritt Ack, Ablage und Publishes; ein Fehlschlag rollt alles zurück.
- [ ] Der `auswerten`-Adapter bindet **keine** Outbox-Consumer-Mechanik — `architecture`-Test grün.
- [ ] Der Test läuft ohne Datei-, Netz- oder echten Datenbankzugriff.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

- Blocked by [0039](0039-io-fenstern-end-to-end-ueber-generische-outboxconsumer.md) — `fenstern` muss
  in `io` verdrahtet sein, bevor `auswerten` an dessen Nachrichten hängt.
- Blocked by [0043](0043-historie-vollstaendig-handler-und-fenster-uebergreifende-query.md) — der
  zweite Handler ist Teil dessen, was end-to-end getrieben wird.
