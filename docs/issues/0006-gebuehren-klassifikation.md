---
id: "0006"
title: "Gebühren-Klassifikation"
status: blocked
---

# Gebühren-Klassifikation

**Typ:** AFK

## What to build

Dritter Teil des Analyse-Kerns: Bank-Gebühren (z. B. Buchungstext `ENTGELT`)
werden gesondert klassifiziert und summiert, damit sichtbar wird, was die Bank
selbst einzieht.

Wichtige Unterscheidung: die **Umbuchungskosten** — die Gebühr **für** eine
Umbuchung — sind ein echter Abfluss (Gebühr) und werden als solcher
ausgewiesen, anders als die Umbuchung selbst, die in Slice 0004 genettet wird.
Diese Unterscheidung setzt das Umbuchung-Netting voraus, um „die Umbuchung"
von „die Gebühr für die Umbuchung" trennen zu können.

## Acceptance criteria

- [ ] Eine Buchung mit Buchungstext `ENTGELT` (oder äquivalentem
      Gebühren-Marker) wird als `Buchungsart`-Gebühr klassifiziert, nicht als
      normaler Abfluss.
- [ ] Gebühren werden über den Auswertungszeitraum summiert und je Konto
      ausweisbar.
- [ ] Eine Umbuchung zwischen eigenen Konten mit begleitender Gebührenbuchung:
      die Umbuchung selbst wird genettet (Slice 0004), die Gebühr dafür bleibt
      als echter Abfluss sichtbar — Test deckt beide Seiten ab.
- [ ] Test deckt den DTO-Vertrag ab: Buchungs-Liste rein, Gebühren-Summen
      raus, ohne Datei-/Netzzugriff.

## Blocked by

- Blocked by [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blocked by [0004](0004-umbuchung-netting.md)
