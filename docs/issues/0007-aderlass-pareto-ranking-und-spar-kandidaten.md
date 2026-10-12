---
id: "0007"
title: "Aderlass, Pareto-Ranking und Spar-Kandidaten"
status: open
---

# Aderlass, Pareto-Ranking und Spar-Kandidaten

**Typ:** AFK

> GitHub-Issue: [#95](https://github.com/mbalzert1978/bank_auswertung/issues/95)

> **Verortung ([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).**
> Analyzer-Baustein **unter dem `auswerten`-Slice**, die **pro-Fenster**-Reichweite: Cashflow/Pareto/
> Spar-Kandidaten **innerhalb** eines `Auswertungsfenster`s (Handler auf `FensterGeschlossen`). Der
> Pareto-Top-N darf optional den **Kostentreiber-Filter** der fenster-übergreifenden Wiederkehr-Query
> ([0005](0005-wiederkehr-erkennung.md)) speisen.

## Was zu bauen ist

Vierter und letzter Teil des Analyse-Kerns, die drei eng gekoppelten
„Aderlass/Ranking/Sparen"-Fähigkeiten in einem Slice: der Aderlass/Cashflow je
Konto und Monat, ein Pareto-Ranking der größten Abflüsse darüber, und
strukturell gefundene Spar-Kandidaten, die direkt aus dem Ranking abgeleitet
werden (deshalb zusammen — Spar-Kandidaten sind kein eigenständiger
Berechnungsschritt, sondern eine Sicht auf die Pareto-Spitze).

Rechnet auf bereits genetteten Abflüssen (Slice 0004) plus den klassifizierten
Treibern aus Wiederkehr (Slice 0005) und Gebühren (Slice 0006) — das Ranking
soll wiederkehrende Kosten und Gebühren als Kandidaten mit einbeziehen, nicht
nur Einzelbuchungen.

## Abnahmekriterien

- [ ] Cashflow/Aderlass wird je eigenem Konto und je Monat berechnet, über den
      gesamten aus der CSV verfügbaren Zeitraum.
- [ ] Ein Pareto-Ranking sortiert die größten Abflüsse (inkl. wiederkehrender
      Treiber aus Slice 0005 und Gebühren aus Slice 0006) nach Anteil an der
      Gesamtsumme.
- [ ] Spar-Kandidaten werden strukturell aus der Pareto-Spitze abgeleitet
      (z. B. teuerste wiederkehrende Treiber, größte Preissteigerungen) —
      nicht durch das LLM vorgeschlagen.
- [ ] Test: eine genettete Umbuchung taucht **nicht** im Cashflow oder im
      Pareto-Ranking als Abfluss auf.
- [ ] Test deckt den DTO-Vertrag ab: Buchungs-/Wiederkehr-/Gebühren-Ergebnisse
      rein, Cashflow- + Ranking- + Spar-Kandidaten-DTOs raus, ohne
      Datei-/Netzzugriff.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0004](0004-umbuchung-netting.md)
- Blockiert durch [0005](0005-wiederkehr-erkennung.md)
- Blockiert durch [0006](0006-gebuehren-klassifikation.md)
- Blockiert durch [0040](0040-auswerten-triade-skelett-pro-fenster-reichweite.md) — Cashflow/Pareto
  rechnen in der pro-Fenster-Reichweite; das Skelett liefert bereits den Monats-Cashflow.
