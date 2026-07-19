---
id: "0004"
title: "Umbuchung-Netting"
status: blocked
---

# Umbuchung-Netting

**Typ:** AFK

> **Verortung ([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).**
> Analyzer-Baustein **unter dem `auswerten`-Slice**. Läuft **in** `auswerten` auf den obfuskierten
> Buchungen eines `Auswertungsfenster`s; `Gegenkonto::Eigen|Fremd` wird **token-orientiert** aufgelöst
> (Gegenkonto-`KONTO_x` gegen die seed-tokenisierte Menge der eigenen Konten, ADR-0013 F) — **nicht**
> mehr vom CSV-Adapter (der frühere „Slice 0003" ist entfallen). Das Netting speist beide Reichweiten:
> die Pro-Fenster-Analyse **und** die fenster-übergreifende Wiederkehr, die auf **genetteten** Abflüssen
> liest.

## Was zu bauen ist

Erster Teil des Analyse-Kerns (Baureihenfolge-Schritt 3), black-box am
DTO-Vertrag testbar ohne Datei und ohne Netz: Umbuchungen zwischen Markus'
eigenen Konten werden erkannt und aus der „Wo bluten wir"-Analyse
herausgenettet — eigenes Geldverschieben zählt nicht als Abfluss. Genettete
Umbuchungen bleiben trotzdem als eigener Blick auf die Geldflüsse zwischen den
eigenen Konten sichtbar (zwei Seiten derselben Sache, nicht eine gelöschte).

Matching-Logik: erkennt Buchungspaare, bei denen `Gegenkonto::Eigen` steht
(_welche_ Gegen-IBAN eigen ist, hat der Adapter in Slice 0003 schon
aufgelöst — dieser Slice matcht nur noch darauf).

## Abnahmekriterien

- [ ] Ein Buchungspaar zweier eigener Konten (Ausgang von Konto A, Eingang bei
      Konto B, `Gegenkonto::Eigen`) wird als Umbuchung erkannt und aus der
      Abfluss-Summe herausgenettet.
- [ ] Die erkannte Umbuchung bleibt über eine eigene Geldfluss-Sicht zwischen
      den eigenen Konten sichtbar (Test: Umbuchung erscheint dort, obwohl sie
      aus der Abfluss-Summe verschwindet).
- [ ] Eine normale Ausgangs-Buchung an ein fremdes Konto (`Gegenkonto::Fremd`)
      wird **nicht** genettet und bleibt voll in der Abfluss-Summe.
- [ ] Test deckt den DTO-Vertrag ab: eine Liste synthetischer `Buchung`-DTOs
      rein, Netting-Ergebnis-DTO raus — ohne Datei- oder Netzzugriff.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0042](0042-eigene-konten-config-und-gegenkonto-auf-token.md) — das Netting
  matcht auf `Gegenkonto::Eigen`, das dort token-orientiert aufgelöst wird.
