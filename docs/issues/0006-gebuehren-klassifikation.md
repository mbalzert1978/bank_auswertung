---
id: "0006"
title: "Gebühren-Klassifikation"
status: closed
---

# Gebühren-Klassifikation

**Typ:** AFK

> **Verortung ([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).**
> Analyzer-Baustein **unter dem `auswerten`-Slice**, die **pro-Fenster**-Reichweite: klassifiziert die
> Gebühren **innerhalb** eines `Auswertungsfenster`s (Handler auf `FensterGeschlossen`). „Slice 0004"
> unten meint den Netting-Baustein [0004](0004-umbuchung-netting.md), der ebenfalls unter `auswerten`
> hängt.

## Was zu bauen ist

Dritter Teil des Analyse-Kerns: Bank-Gebühren (z. B. Buchungstext `ENTGELT`)
werden gesondert klassifiziert und summiert, damit sichtbar wird, was die Bank
selbst einzieht.

Wichtige Unterscheidung: die **Umbuchungskosten** — die Gebühr **für** eine
Umbuchung — sind ein echter Abfluss (Gebühr) und werden als solcher
ausgewiesen, anders als die Umbuchung selbst, die in Slice 0004 genettet wird.
Diese Unterscheidung setzt das Umbuchung-Netting voraus, um „die Umbuchung"
von „die Gebühr für die Umbuchung" trennen zu können.

## Abnahmekriterien

- [x] Eine Buchung mit Buchungstext `ENTGELT` (oder äquivalentem
      Gebühren-Marker) wird als `Buchungsart`-Gebühr klassifiziert, nicht als
      normaler Abfluss.
- [x] Gebühren werden über den Auswertungszeitraum summiert und je Konto
      ausweisbar.
- [x] Eine Umbuchung zwischen eigenen Konten mit begleitender Gebührenbuchung:
      die Umbuchung selbst wird genettet (Slice 0004), die Gebühr dafür bleibt
      als echter Abfluss sichtbar — Test deckt beide Seiten ab.
- [x] Test deckt den DTO-Vertrag ab: Buchungs-Liste rein, Gebühren-Summen
      raus, ohne Datei-/Netzzugriff.

## Wie es gebaut wurde (2026-07-31)

- **Überlagerung, keine Aufteilung — der entscheidende Unterschied zu 0004.** „Nicht als
  normaler Abfluss" heißt **nicht**, dass die Gebühr den Cashflow verlässt. Sie ist echtes,
  verlorenes Geld und bleibt in den Monats-Buckets stehen; die Gebühren-Sicht hebt sie
  zusätzlich heraus. Die Umbuchung dagegen **teilt** den Cashflow (disjunkt, zusammen
  vollständig), weil sie kein Abfluss ist. Wer die beiden Ergebnis-Listen gleich behandelt,
  zählt bei den Gebühren doppelt — im Code, im Schema und in den Specs steht das ausdrücklich
  dabei.
- **Die Gebühren lesen aus derselben Menge wie der Cashflow** (also nach dem Netting). So gilt
  „Gebühren ⊆ Monats-Buckets" von Bauart wegen statt nur in der Praxis.
- **Klassifikation als reine Domänenfunktion** `BookingKind::of(buchungstext)` — dieselbe Linie
  wie beim `Fingerabdruck` (ADR-0004): Fachwissen der Domäne, keine Konfiguration. Ein
  Summentyp statt eines `ist_gebuehr`-Flags, damit Zins/Storno später danebenpassen.
- **Je Monat und belastetem Konto**, nicht nur je Monat — das ist die Zelle „je Konto
  ausweisbar". Das Konto ist das **Auftragskonto** (eine Entgeltzeile hat keine Gegenseite,
  die Bank bucht direkt), es steht als opakes `KONTO_…`-Token da.
- **Zwei Zellen mussten nachwachsen**: `ordering_account` und `booking_text` lagen bereits in
  der `obfuskiert`-Ablage, fehlten aber in der `SELECT`-Liste, die `auswerten` liest. Da dessen
  Lese-Seam ohnehin auf `obfuskiert` joint, blieb `fenstern` **unangetastet** — kein
  Durchreichen durch die Pipeline nötig.
- **Persistenz:** eine siebte Tabelle `window_evaluation_fees`. Ohne `CHECK` auf `account`:
  das Token-Vokabular ist nicht endlich, es wächst mit jedem eingelesenen Konto.
- **Offene HITL-Lücke:** die Marker-Liste (`ENTGELT`, `GEBUEHR`, `GEBÜHR`) stammt aus dem
  Vokabular der Fixture-Auszüge und aus allgemeiner Kenntnis — **nicht** aus einer Sichtung von
  Markus' echten Auszügen, anders als die Gläubiger-IDs der Durchleiter (0047). Bewusst
  konservativ: ein `ABSCHLUSS` allein steht auch am Rechnungsabschluss, der Zinsen bucht.
  Lieber eine Gebühr übersehen als eine Nicht-Gebühr etikettieren.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0004](0004-umbuchung-netting.md)
- Blockiert durch [0040](0040-auswerten-triade-skelett-pro-fenster-reichweite.md) — rechnet in der
  pro-Fenster-Reichweite von `auswerten`.
