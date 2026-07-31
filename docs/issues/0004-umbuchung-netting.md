---
id: "0004"
title: "Umbuchung-Netting"
status: closed
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

- [x] Ein Buchungspaar zweier eigener Konten (Ausgang von Konto A, Eingang bei
      Konto B, `Gegenkonto::Eigen`) wird als Umbuchung erkannt und aus der
      Abfluss-Summe herausgenettet.
- [x] Die erkannte Umbuchung bleibt über eine eigene Geldfluss-Sicht zwischen
      den eigenen Konten sichtbar (Test: Umbuchung erscheint dort, obwohl sie
      aus der Abfluss-Summe verschwindet).
- [x] Eine normale Ausgangs-Buchung an ein fremdes Konto (`Gegenkonto::Fremd`)
      wird **nicht** genettet und bleibt voll in der Abfluss-Summe.
- [x] Test deckt den DTO-Vertrag ab: eine Liste synthetischer `Buchung`-DTOs
      rein, Netting-Ergebnis-DTO raus — ohne Datei- oder Netzzugriff.

## Wie es gebaut wurde (2026-07-31)

- **Netting je Zeile, nicht je Paar.** Der Ticket-Text spricht vom „Buchungspaar",
  seine eigene Matching-Logik aber nur von `Gegenkonto::Eigen` — und das ist auch das
  Richtige: auf das zweite Bein zu warten hieße, jede Umbuchung als Ausgabe zu zählen,
  deren Gegenstück in einem anderen Fenster, einem nicht eingelesenen Auszug oder
  jenseits des Monatswechsels liegt. Seit die Auflösung **auf Token** arbeitet
  ([0042](0042-eigene-konten-config-und-gegenkonto-auf-token.md)) steht „das Gegenkonto
  ist meines" schon an der einzelnen Zeile. Der Paar-Fall ist trotzdem als Spec belegt
  (`beide_beine_eines_umbuchungspaares_werden_genettet`).
- **Beide Richtungen fallen heraus**, nicht nur der Abfluss — sonst sähe das Sparkonto
  wie eine Einnahmequelle aus.
- **Die Geldfluss-Sicht** ist `InternalTransfer`: dieselbe Form wie ein Cashflow-Bucket
  (Monat + zwei vorzeichenlose Spalten), zusätzlich die **Rolle des Gegenkontos**. Dass
  die Form dieselbe ist, ist die Zusage „ausgeschlossen, aber sichtbar": brutto = genettet
  plus umgebucht. Kein verrechneter Saldo — das Vorzeichen lebt in der Richtung (ADR-0005).
- **Die Gebühr _für_ eine Umbuchung** hat kein Gegenkonto und bleibt darum im Abfluss
  (CONTEXT.md, „Umbuchungskosten" — der Hauptverdächtige des Projekts).
- **Persistenz:** eine sechste Tabelle `window_evaluation_transfers` im `auswerten`-Store
  (`role` per `CHECK` aufs Vier-Wort-Vokabular). Kein Seam Richtung Slice geändert.
- **Am echten Lauf belegt:** der Juni-Abfluss sinkt von 173060 auf 143060, die fehlenden
  30000 stehen als Umbuchung aufs Sparkonto daneben — 173060 = 143060 + 30000.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0042](0042-eigene-konten-config-und-gegenkonto-auf-token.md) — das Netting
  matcht auf `Gegenkonto::Eigen`, das dort token-orientiert aufgelöst wird.
