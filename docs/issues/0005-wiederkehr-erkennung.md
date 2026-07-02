---
id: "0005"
title: "Wiederkehr-Erkennung"
status: blocked
---

# Wiederkehr-Erkennung

**Typ:** AFK

## Was zu bauen ist

Zweiter Teil des Analyse-Kerns: deterministische Erkennung wiederkehrender
Abflüsse (Abos, Daueraufträge) — inkl. Jahreskosten (jährlich statt monatlich
abgebuchte Abos) und Preissteigerungen bei ansonsten gleichbleibenden
wiederkehrenden Abflüssen.

Vertieft hier `Fingerabdruck` (D2) um die Zahlungsintermediär-Erkennung:
PayPal/Klarna/Amazon Payments etc. dürfen nicht alle dahinterliegenden
Händler zu einem Klumpen zusammenfassen — die Wiederkehr muss am echten
Händler-Token hängen, nicht am Durchleiter. Das ist Domänenwissen (nicht
Adapter-Konzern) und wirkt sich auch auf die spätere Kategorisierung aus
(Textwahl Verwendungszweck vs. Empfänger bei Intermediären, siehe Slice 0011).

Arbeitet auf bereits genetteten Abflüssen (Slice 0004) — eine wiederkehrende
Umbuchung an ein eigenes Konto zählt nicht als wiederkehrender Kosten-Treiber.

## Abnahmekriterien

- [ ] Eine Serie gleichartiger monatlicher Ausgangs-Buchungen (gleicher
      Fingerabdruck, ähnlicher Betrag, regelmäßiger Abstand) wird als
      wiederkehrender Abfluss erkannt.
- [ ] Eine jährlich abgebuchte Buchung (12-Monats-Abstand) wird ebenfalls als
      wiederkehrend erkannt, nicht übersehen.
- [ ] Eine Preissteigerung bei sonst gleichbleibendem wiederkehrendem Abfluss
      (gleicher Fingerabdruck, gestiegener Betrag ab einem Stichtag) wird
      erkannt und ausgewiesen.
- [ ] Zwei verschiedene Händler, die beide über denselben
      Zahlungsintermediär abgerechnet werden, ergeben **unterschiedliche**
      Fingerabdrücke/Wiederkehr-Gruppen — Test mit mindestens zwei
      Fixture-Händlern hinter demselben Intermediär.
- [ ] Eine wiederkehrende Umbuchung an ein eigenes Konto erscheint **nicht**
      in der Wiederkehr-Liste der Kosten-Treiber.
- [ ] Test deckt den DTO-Vertrag ab: Buchungs-Liste rein, Wiederkehr-Ergebnis
      raus, ohne Datei-/Netzzugriff.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0004](0004-umbuchung-netting.md)
