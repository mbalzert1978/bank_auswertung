---
id: "0037"
title: fenstern — Selbst-Fortsetzung über mehrere Seiten
status: blocked
---

# fenstern — Selbst-Fortsetzung über mehrere Seiten

**Typ:** AFK

## What to build

Der Fenster-Schritt entscheidet nach dem Ablegen, **wie es weitergeht** — und tut das mit **einem**
Mechanismus statt zweier ([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md),
Entscheidung B, Präzisierung 2):

Der **nächste Anker ist der jüngste echte Buchungstag unter der Fensterunterkante**
(`max(booking_day < from)`). Liefert die Abfrage einen Tag, publiziert der Schritt das nächste
**`AdvanceWindow`** mit diesem Anker und die Kette läuft weiter; liefert sie **nichts**, publiziert
er stattdessen **`HistorieVollständig`** mit der Lauf-Spanne. Kein Zähler, kein Boolean, kein
Watermark-Sonderfall — dieselbe Abfrage-Art wie beim Initial-Anker, nur mit Untergrenze.

Das trägt zwei Eigenschaften, die der ältere `from − 1`-Entwurf **nicht** garantierte: **jedes
Fenster sitzt auf echten Daten**, also gibt es kein leeres Aggregat (Datenlücken — auch größer als
eine Seitengröße — werden **übersprungen** statt in leere Fenster gegossen), und die Fenster
**überlappen nicht**, also landet jede Buchung in genau einem Fenster. Die Terminierung ist
natürlich, weil der Anker streng monoton sinkt.

Ablage und **beide** Publishes committen **atomar zusammen oder gar nicht**; die Unit-of-Work trägt
mehrere Publishes je Transaktion bereits (`einlesen` publiziert schon N je Lauf).

## Acceptance criteria

- [ ] Eine Historie über mehrere Seiten erzeugt **je Seite** ein `Auswertungsfenster` samt
      `FensterGeschlossen` und **genau ein** abschließendes `HistorieVollständig`.
- [ ] Eine Datenlücke, die größer ist als eine Seitengröße, wird übersprungen — es entsteht **kein
      leeres** Fenster und die Kette bricht nicht ab.
- [ ] Die Fenster überlappen nicht; jede Buchung erscheint in **genau einem** Fenster.
- [ ] Die Kette terminiert garantiert (der Anker sinkt streng monoton, die Abfrage versiegt).
- [ ] Ablage + beide Publishes sind **eine** Einheit: schlägt eines fehl, ist nichts abgelegt und
      nichts publiziert — es entsteht keine gegabelte Kette.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

Blocked by [0036](0036-fenstern-triade-ein-auswertungsfenster-materialisieren.md) — die Fortsetzung
setzt den Ein-Seiten-Pfad (Aggregat bauen, ablegen, publizieren) voraus.
