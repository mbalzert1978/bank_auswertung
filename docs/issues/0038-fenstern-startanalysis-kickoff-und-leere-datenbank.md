---
id: "0038"
title: fenstern — StartAnalysis-Kickoff und der leere Datenbestand
status: blocked
---

# fenstern — StartAnalysis-Kickoff und der leere Datenbestand

**Typ:** AFK

## What to build

Der **zweite einzweckige Handler** des Slice — der Anstoß
([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md), Entscheidung B/D und
die Leer-Konsequenz):

**`StartAnalysis`** ist ein **feldloses** Auslöser-Kommando — derselbe Kickoff-Mechanismus wie bei
`einlesen`. Dem Slice ist egal, **wer** es einreiht (heute manuell, später der Broker beim
Leerlaufen des Imports). Der Start-Handler liest **einmalig** den Anker über den **`WindowAnchor`**-
Port — die Implementierung ist `max(booking_day)`, also **datenrelativ** und deterministisch — und
publiziert `AdvanceWindow` mit diesem Anker.

Warum **zwei** Kommandos und nicht eines mit optionalem Anker: der **Typ trägt die Absicht**. Der
feldlose `StartAnalysis` startet, `AdvanceWindow` trägt **garantiert** einen Anker; der illegale
Zustand „leerer Anker" ist damit **unkonstruierbar** statt geprüft (ADR-0005). Jeder Handler hat
**eine** Aufgabe.

**Der leere Datenbestand ist kein Fehler.** Liefert der Anker-Port keinen Buchungstag, endet der
Lauf mit **Erfolg**: es wird **nichts** publiziert und `auswerten` **nicht** angestoßen — Erschöpfung
schon bei Schritt 0, symmetrisch zum normalen Ketten-Ende. Kein Dead-Letter (den gibt es nicht),
kein „nichts auszuwerten"-Fehler. Nebeneffekt, der zählt: weil so **kein** `HistorieVollständig` mit
leerer Spanne entsteht, bleibt dessen Lauf-Spanne garantiert nicht entartet.

## Acceptance criteria

- [ ] Ein `StartAnalysis` führt zu **genau einem** `AdvanceWindow`, dessen Anker der jüngste
      Buchungstag des Bestands ist.
- [ ] Bei **leerem** Bestand endet `StartAnalysis` **erfolgreich**, publiziert **nichts** und stößt
      `auswerten` nicht an — es entsteht insbesondere kein `HistorieVollständig`.
- [ ] Der Anker ist im Fenster-Kommando **nicht optional**; ein „leerer Anker" ist unkonstruierbar.
- [ ] Der Anker liegt hinter einem Port — die Specs faken ihn mit festem Datum, ohne Uhr.
- [ ] Der Slice hat **zwei** Kommandotypen und **zwei** einzweckige Handler.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

Blocked by [0036](0036-fenstern-triade-ein-auswertungsfenster-materialisieren.md) — der Anstoß
publiziert `AdvanceWindow`, das es dafür geben muss.
