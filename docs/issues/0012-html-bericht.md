---
id: "0012"
title: "HTML-Bericht"
status: open
---

# HTML-Bericht

**Typ:** AFK

> GitHub-Issue: [#99](https://github.com/mbalzert1978/bank_auswertung/issues/99)

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

> **Vorbedingung (Entscheidung des Orchestrators, 2026-10-12):** das Kategorie-Register aus
> [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md) ist die **einzige Wahrheit** für
> Kategorie-Etiketten. Der Bericht liest die Kategorie je Treiber **ausschließlich** über
> `io::auswerten::SqliteCategoryRegistry::categories_of` (fehlt ein Eintrag: „unbekannt"). Die
> Etiketten im Fenster-Ergebnis (`window_evaluation_categories`) sind nur der Mitschnitt dessen, was
> das Orakel in jenem Lauf beitrug — **keine** Quelle für den Bericht.

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
- [ ] Ist `RunReport::incomplete() > 0`, nennt der Bericht sichtbar, wie viele Buchungen der
      Analyse fehlen ([ADR-0017](../adr/0017-dead-letter-als-status-spalte-der-outbox.md)).
- [ ] Jedes Kategorie-Etikett wird im HTML **escaped**: es stammt aus einer Chat-Antwort
      ([0011](0011-kategorisierer-chat-payload-adapter.md)) und ist nur auf Länge, Einzeiligkeit und
      Steuerzeichen geprüft, nicht auf HTML.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0004](0004-umbuchung-netting.md)
- Blockiert durch [0005](0005-wiederkehr-erkennung.md)
- Blockiert durch [0006](0006-gebuehren-klassifikation.md)
- Blockiert durch [0007](0007-aderlass-pareto-ranking-und-spar-kandidaten.md)
