---
id: "0087"
title: "CSV-Fixtures konsolidieren und als unverkennbar erfunden kennzeichnen"
status: open
---

# CSV-Fixtures konsolidieren und als unverkennbar erfunden kennzeichnen

**Typ:** AFK

> GitHub-Issue: [#104](https://github.com/mbalzert1978/bank_auswertung/issues/104)

## What to build

Der `einlesen`-CSV-Fixture-Bestand ist gewachsen: acht Dateien, die denselben Header
wiederholen und dieselbe Handvoll Werte in leicht abweichenden Schreibweisen tragen. Zwei
Konventionen stehen unversöhnt nebeneinander — die älteren Fixtures benutzen bekannte
Beispiel-IBANs echter Banken, die jüngste Lauf-Fixture ausschließlich **strukturell
unmögliche** IBANs mit Prüfziffer `00`. Aus derselben Zeit stammen unterschiedliche
Empfänger-Namen, teils reale Firmennamen, teils erfundene.

Das ist keine Kosmetik, sondern die Durchsetzbarkeit der wichtigsten Zusage des Projekts:
**in diesem Repository steht kein einziger echter Kontodatensatz.** Solange die Fixtures
uneinheitlich sind, lässt sich das nur durch Hinsehen behaupten; mit einer durchgehaltenen
Konvention lässt es sich prüfen.

Zu tun:

- **Eine Konvention für erfundene Daten** festschreiben und auf alle Fixtures ziehen: IBANs
  mit einer Prüfziffer, die es real nicht geben kann, Gläubiger-IDs im selben Schema,
  ausschließlich frei erfundene Empfänger- und Verwendungszweck-Texte (keine realen
  Firmennamen), erfundene BICs.
- **Den Bestand entdoppeln**: die Fixtures unterscheiden sich meist in genau einer Zelle
  (fehlendes Pflichtfeld, undeutbarer Betrag, fremde Währung). Die gemeinsame Grundzeile
  gehört an einen Ort, die Abweichung bleibt sichtbar der Punkt der jeweiligen Datei.
- **Die Konvention neben den Fixtures dokumentieren**, damit die nächste Fixture sie erbt.
- **Einen Wächter** ergänzen, der eine Zeile meldet, die von der Konvention abweicht — so
  wird eine versehentlich eingefügte echte IBAN ein roter Test statt eines Review-Zufalls.

## Acceptance criteria

- [ ] Alle CSV-Fixtures folgen einer einzigen, dokumentierten Konvention für erfundene Konten, Empfänger und Referenzen.
- [ ] Keine Fixture trägt mehr eine real existierende IBAN, einen realen Firmennamen oder eine reale Gläubiger-ID.
- [ ] Die Wiederholung zwischen den Fixtures ist aufgelöst; jede Datei zeigt sichtbar genau den Fall, für den sie existiert.
- [ ] Ein Test schlägt fehl, sobald eine Fixture-Zeile von der Konvention abweicht.
- [ ] Alle bestehenden `einlesen`- und `io`-Tests bleiben grün und prüfen weiterhin dasselbe Verhalten.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- None — can start immediately.
