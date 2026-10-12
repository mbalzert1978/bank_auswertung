---
id: "0011"
title: "Kategorisierer Chat-Payload-Adapter"
status: open
---

# Kategorisierer Chat-Payload-Adapter

**Typ:** AFK

> GitHub-Issue: [#98](https://github.com/mbalzert1978/bank_auswertung/issues/98)

## Was zu bauen ist

Der `Kategorisierer`-Port (outbound), Chat-Payload-Adapter zuerst
(Baureihenfolge-Schritt 5): exportiert **nur die der Domäne noch unbekannten**
Fingerabdrücke (Abgleich gegen das Kategorie-Register aus Slice 0009) als JSON
(`KategorieAnfrage`: Fingerabdruck + `ObfuskierterText`), der Nutzer gibt das
in einen Chat, die Antwort (`KategorieAntwort`: Fingerabdruck → Kategorie)
wird zurück importiert und ins Kategorie-Register übernommen.

`KategorieAntwort` schlüsselt ausschließlich auf dem Fingerabdruck, nie auf
dem Text — Re-Identifikation ist damit strukturell ausgeschlossen. Bei
Zahlungsintermediären muss die exportierte `ObfuskierterText`-Nutzlast aus dem
Verwendungszweck stammen, nicht aus dem (uninformativen) Empfänger-Namen des
Intermediärs — das nutzt die Intermediär-Erkennung aus Slice 0005.

> **Vorbedingung (Entscheidung des Orchestrators, 2026-10-12):** das Kategorie-Register aus
> [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md) ist die **einzige Wahrheit** für
> Kategorie-Etiketten. Der Import schreibt **ausschließlich** über
> `io::auswerten::SqliteCategoryRegistry::learn(&[CategoryAnswerRow])` — alles oder nichts, und nur
> Fingerabdrücke, die eine Auswertung schon geführt hat (`window_evaluation_fingerprints`); ein
> erfundener Schlüssel, etwa ein Klarname aus der Chat-Antwort, wird redigiert abgewiesen
> (`RejectCause::UnknownFingerprint`). Welche Fingerabdrücke unbekannt sind, beantwortet derselbe
> Register-Seam (`known_among`).

## Abnahmekriterien

- [ ] Export enthält ausschließlich Fingerabdrücke, die im Kategorie-Register
      noch **keine** Kategorie haben — bereits bekannte Fingerabdrücke werden
      nicht erneut exportiert.
- [ ] Export-Payload enthält für Buchungen mit erkanntem
      Zahlungsintermediär den Verwendungszweck (nicht den Empfänger-Namen)
      als `ObfuskierterText`.
- [ ] Import einer `KategorieAntwort`-Datei ordnet Kategorien korrekt über
      den Fingerabdruck zu und schreibt sie ins Kategorie-Register (Slice
      0009); eine erneute Analyse zeigt die gelernte Kategorie.
- [ ] Test: kein Feld in Export oder Import erlaubt eine Zuordnung über den
      Freitext statt über den Fingerabdruck — eine Antwort ohne passenden
      Fingerabdruck wird verworfen, nicht texto-heuristisch zugeordnet.
- [ ] Test deckt den vollständigen Round-Trip ab: unbekannte Fingerabdrücke →
      Export-JSON → (kanonische Fixture-Antwort) → Import → aktualisiertes
      Kategorie-Register.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0005](0005-wiederkehr-erkennung.md) (Zahlungsintermediär-Erkennung
  für die Textwahl im Export)
- Blockiert durch [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)
- Blockiert durch [0010](0010-scrubbing-regeln-fuer-obfuskiertertext.md)
