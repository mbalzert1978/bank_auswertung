---
id: "0012"
title: "HTML-Bericht"
status: blocked
---

# HTML-Bericht

**Typ:** AFK

## Was zu bauen ist

Der `Bericht`-Port (outbound), HTML zuerst (Baureihenfolge-Schritt 6): zeigt
die **Analyse der Domäne** — Umbuchungen (Slice 0004), Wiederkehr (Slice
0005), Gebühren (Slice 0006), Aderlass/Pareto/Spar-Kandidaten (Slice 0007) —
nicht „was das LLM sagte". Kategorie ist nur ein Attribut einer `Buchung`,
kein Berichtsinhalt für sich.

Bewusst **nicht** blockiert durch Speicher (Slice 0009) oder Kategorisierer
(Slice 0011): Bericht und Speicher sind unabhängige hexagonale Ports, und der
Lackmustest verlangt, dass das Programm auch ganz ohne LLM wertvoll bleibt —
der Bericht muss also auch mit unkategorisierten Buchungen sinnvoll
funktionieren.

## Abnahmekriterien

- [ ] HTML-Bericht rendert Umbuchungs-Übersicht, Wiederkehr-Liste (inkl.
      Jahreskosten/Preissteigerungen), Gebühren-Summen, Aderlass/Cashflow je
      Konto & Monat, Pareto-Ranking und Spar-Kandidaten aus den jeweiligen
      Analyse-Ergebnissen.
- [ ] Buchungen ohne Kategorie erscheinen im Bericht (z. B. als „unbekannt"),
      der Bericht bricht nicht und verliert keine Analyse-Inhalte, wenn keine
      einzige Buchung kategorisiert ist.
- [ ] Test/Fixture belegt: der Bericht zeigt errechnete Domänen-Werte
      (Summen, Rankings), nicht einen roh durchgereichten LLM-Text.
- [ ] Bericht lässt sich aus reinen Fixture-Analyse-DTOs erzeugen, ganz ohne
      SQLite-Speicher oder Kategorisierer-Adapter im Testlauf.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0004](0004-umbuchung-netting.md)
- Blockiert durch [0005](0005-wiederkehr-erkennung.md)
- Blockiert durch [0006](0006-gebuehren-klassifikation.md)
- Blockiert durch [0007](0007-aderlass-pareto-ranking-und-spar-kandidaten.md)
