---
id: "0050"
title: "Money/Calendar-Decode-Seams real (io, hinter einlesen)"
status: closed
---

# Money/Calendar-Decode-Seams real (io, hinter einlesen)

**Typ:** AFK

## What to build

Die echte Erfüllung der `Money`- und `Calendar`-Seams des einlesen-Adapters in `io`, als Ersatz der
In-Memory-Fakes — **rein, kein I/O, kein SQLite**. Der risikoärmste erste echte io-Schnitt; er übt die
Seam-/Bridge-Mechanik, ohne Persistenz zu berühren, und hängt darum **nicht** an der Stack-Entscheidung
([0049](0049-persistenz-stack-rusqlite-synchron-kein-sqlx.md)).

- **`Money`** dekodiert die deutsche Betragsdarstellung (Tausenderpunkt / Dezimalkomma, führendes
  Vorzeichen) zu einem dekodierten Betrag. Ein **undeutbarer** vorhandener Wert ergibt `None`, was das
  Aggregat terminal auf `AmountInvalid` mappt.
- **`Calendar`** dekodiert das deutsche Datum (TT.MM.JJJJ) zu den `CalendarDay`-Teilen. Die **Seam
  besitzt die Kalendergültigkeit** — `CalendarDay::from_parts` ist infallibel; ein undeutbares Feld
  gibt `None` zurück, das die Domäne terminal auf `BookingDayInvalid`/`ValueDateInvalid` mappt.

Format-Mechanik lebt hier hinter dem Seam, **nicht in der Domäne**
([ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md)): die Domäne demandiert fertig
dekodierte Werte, sie validiert kein Format.

## Acceptance criteria

- [ ] `Money`- und `Calendar`-Seams sind in `io` mit echter deutscher Zahl-/Datum-Dekodierung erfüllt;
      die bisherigen Fakes sind ersetzt.
- [ ] Ein undeutbarer Betrag → `None` → terminaler `LineError`; ein undeutbares Datum ebenso terminal —
      über den einlesen-Bridge-Weg (Fluent-TestApi) verifiziert, nicht als io-Unit-Test.
- [ ] Neue Fremd-Deps (falls nötig) liegen **nur in `io`**; die Domäne bleibt dependency-frei;
      `cargo test -p architecture` bleibt grün.
- [ ] Die Domäne ist unverändert — die Seam dekodiert, die Domäne demandiert.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`,
      `cargo test -p architecture` und `cargo doc --no-deps --workspace` sind grün.

## Blocked by

None - can start immediately. (Hängt bewusst **nicht** an
[0049](0049-persistenz-stack-rusqlite-synchron-kein-sqlx.md) — reiner Dekodier-Schnitt ohne
Persistenz.)
