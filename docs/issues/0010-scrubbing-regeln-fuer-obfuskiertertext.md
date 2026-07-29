---
id: "0010"
title: "Scrubbing-Regeln für ObfuskierterText"
status: open
---

# Scrubbing-Regeln für ObfuskierterText

**Typ:** AFK

## Was zu bauen ist

Vertieft `ObfuskierterText` (D3, Slice 0001 lieferte nur eine minimale
Erstversion) um die eigentlichen Scrubbing-Regeln: in Freitext
(Empfänger/Verwendungszweck) eingebettete Identifikatoren (IBAN-/BIC-Muster,
bekannte Personen-Token) werden **best-effort** gescrubbt, bevor Text
überhaupt als `ObfuskierterText` konstruierbar ist — „ungescrubbt" ist gar
nicht serialisierbar.

Fail-safe: im Zweifel wird eher zu viel als zu wenig gescrubbt. Das Scrubbing
ist **heuristisch, nicht beweisbar vollständig** — das ist eine ehrliche
Grenze des Modells, keine Lücke, die dieser Slice schließen soll (siehe PRD
„Out of Scope").

## Abnahmekriterien

- [ ] Eine im Verwendungszweck eingebettete IBAN wird vom Konstruktor erkannt
      und ersetzt/entfernt, bevor der Text als `ObfuskierterText` existiert.
- [ ] Eine im Freitext eingebettete BIC wird ebenso gescrubbt.
- [ ] Ein bekanntes Personen-Token im Freitext wird gescrubbt (fail-safe:
      Unsicherheitsfall wird lieber entfernt als riskiert).
- [ ] Test-Suite mit mehreren realistischen (aber synthetischen) Freitext-
      Beispielen, die je einen Identifikator-Typ enthalten, belegt die
      Scrubbing-Regeln einzeln.
- [ ] Dokumentiert (Docstring/Kommentar am Konstruktor): das Scrubbing ist
      heuristisch und nicht beweisbar vollständig — keine
      Vollständigkeits-Behauptung im Code oder in Tests.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
