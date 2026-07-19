---
id: "0040"
title: auswerten-Triade — die pro-Fenster-Reichweite
status: blocked
---

# auswerten-Triade — die pro-Fenster-Reichweite

**Typ:** AFK

## What to build

Die Triade **`auswerten`** (`_domain`/`_adapter`/`_specs`) und der **erste ihrer zwei** einzweckigen
Handler ([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md), Entscheidung E):

Auf **`FensterGeschlossen`** liest der Handler das **`Auswertungsfenster` per ID** — er bekommt einen
**Zeiger**, keine eingebetteten Daten (uniformes ID-Handoff, ADR-0012) — und rechnet die erste
**periodische Kennzahl innerhalb** des Fensters: den **Cashflow je Monat**. Das Ergebnis wird
abgelegt.

Wichtig für die Begriffe: das Fenster ist die **Seitengröße** des Pagers (~400 Tage), **nicht** der
Berichtszeitraum — der Cashflow wird **innerhalb** des Fensters **pro Monat gebucketet**
(vgl. die markierte Mehrdeutigkeit „Fenster vs. Zeitraum" in [CONTEXT.md](../../CONTEXT.md)).

Damit steht der Pfad, an dem die **Fach-Analyzer** später andocken
([0004](0004-umbuchung-netting.md) Netting, [0006](0006-gebuehren-klassifikation.md) Gebühren,
[0007](0007-aderlass-pareto-ranking-und-spar-kandidaten.md) Pareto) — dieses Ticket liefert das
Gerüst und **eine** lauffähige Kennzahl, nicht die ganze Fachlogik.

Ein unlesbares oder fehlendes Aggregat ist ein **terminaler, redigierter** Fehlschlag (opake ID, nie
ein Rohwert) — kein Retry im Slice.

## Acceptance criteria

- [ ] Ein `FensterGeschlossen` führt dazu, dass das Aggregat **per ID** gelesen und ein
      Pro-Fenster-Ergebnis abgelegt wird.
- [ ] Der Cashflow ist **je Monat innerhalb** des Fensters gebucketet — ein Fenster deckt mehrere
      Monats-Buckets ab.
- [ ] Ein unlesbares/fehlendes Aggregat endet **terminal und redigiert** (opake ID, kein Rohwert).
- [ ] Der Handler bekommt **nur die ID**, nicht das eingebettete Aggregat.
- [ ] Das `_domain`-Crate ist dependency-frei; die Specs sind Black-Box über die TestApi.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

Blocked by [0036](0036-fenstern-triade-ein-auswertungsfenster-materialisieren.md) — es braucht ein
abgelegtes `Auswertungsfenster` und ein `FensterGeschlossen`, das darauf zeigt.
