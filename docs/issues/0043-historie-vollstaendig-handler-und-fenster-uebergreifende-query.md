---
id: "0043"
title: HistorieVollständig-Handler und die fenster-übergreifende Read-Port-Query
status: blocked
---

# HistorieVollständig-Handler und die fenster-übergreifende Read-Port-Query

**Typ:** AFK

## What to build

Der **zweite einzweckige Handler** von `auswerten` — die **fenster-übergreifende** Reichweite
([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md), Entscheidung E):

Auf **`HistorieVollständig`** läuft — **einmal, bei vollständiger Historie** — eine
**Read-Port-Query über alle `Auswertungsfenster` dieses Laufs**. Die im Signal mitgetragene
**Lauf-Spanne** scopt sie auf genau diesen Lauf, damit Aggregate eines früheren Laufs nicht
einfließen. Die Query liefert die **Vorkommen** (`Fingerabdruck`, Richtung, Betrag, Tag) über die
ganze Zeitachse — **synchron, terminal, im Handler**.

Der Trigger ist nötig, weil `auswerten` die letzte Seite **nicht selbst erkennen kann**: der Pager
läuft neu→alt, und nur `fenstern` weiß, wann die Historie erschöpft ist.

**Ausdrücklich kein gespeichertes Log.** Es gibt **keine** Akkumulation und **kein**
`Vorkommens-Log`/Fold-Log — die Aggregate sind die alleinige Wahrheit, die fenster-übergreifende
Sicht ist ein **Read**. Damit entfallen Dedupe/Idempotenz-Maschinerie ersatzlos: eine Query über
unveränderliche Snapshots ist von Natur aus deterministisch, auch bei erneuter Zustellung.

Dieses Ticket liefert die **Reichweite** — die eigentliche Rhythmus-/Wiederkehr-Logik ist
[0005](0005-wiederkehr-erkennung.md) und baut darauf auf.

## Acceptance criteria

- [ ] Ein `HistorieVollständig` löst **genau einen** Lauf der Query aus; ein `FensterGeschlossen`
      löst sie **nicht** aus.
- [ ] Die Query liefert Vorkommen über **alle** Fenster des Laufs, nicht nur über eines.
- [ ] Die **Lauf-Spanne** scopt korrekt: Aggregate eines anderen Laufs fließen nicht ein.
- [ ] Es entsteht **keine** zweite persistierte Sicht (kein Vorkommens-/Fold-Log, keine Akkumulation).
- [ ] Erneute Zustellung desselben Signals liefert **dasselbe** Ergebnis, ohne Dedupe-Logik.
- [ ] Die Vorkommen tragen genug für Rhythmus und Betragsvergleich (Fingerabdruck, Richtung, Betrag,
      Tag).
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

- Blocked by [0037](0037-fenstern-selbst-fortsetzung-mehr-seiten-paging.md) — erst mit mehreren
  Seiten ist „fenster-übergreifend" überhaupt beobachtbar, und `HistorieVollständig` markiert das Ende.
- Blocked by [0040](0040-auswerten-triade-skelett-pro-fenster-reichweite.md) — der zweite Handler
  setzt die Triade und das Lesen eines Aggregats voraus.
