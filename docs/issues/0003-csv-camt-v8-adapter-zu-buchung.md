---
id: "0003"
title: "CSV-CAMT-v8-Adapter zu Buchung"
status: blocked
---

# CSV-CAMT-v8-Adapter zu Buchung

**Typ:** AFK — braucht als Voraussetzung eine echte CSV-CAMT-v8-Beispieldatei
von Markus (mindestens ein Monat), sonst nicht abschließbar.

## What to build

Der `Umsatzquelle`-Port (inbound), CSV-CAMT-v8-Adapter zuerst
(Baureihenfolge-Schritt 2): liest eine echte CSV-CAMT-v8-Exportdatei ein und
mappt jede Zeile auf eine `Buchung` — **obfuskiert beim Mappen**, sodass die
Domäne nur obfuskierte Value Objects erhält.

Im Adapter (nicht in der Domäne): `Auftragskonto` → `KONTO_x` über die
Eigene-Konten-Config; Gegen-IBAN → `Gegenkonto::Eigen|Fremd` (ebenfalls über
die Eigene-Konten-Config, inkl. Konto-Rolle bei `Eigen`); Privatperson im
Empfänger-Feld → `PERSON_x` (First-Seen-Vergabe, für diesen Slice **nur
laufstabil innerhalb eines Programmlaufs** — laufstabile Persistenz über
mehrere Importe hinweg ist bewusst ausgeklammert und kommt mit der
persistenten Pseudonym-Zuordnung); Freitext (Empfänger/Verwendungszweck) →
`ObfuskierterText`. Rohe IBAN/BIC bleiben ausschließlich im Adapter. Der
Adapter sichert eine einheitliche Währung über die gesamte Datei zu — die
Domäne summiert keine gemischten Währungen.

Ein realer, dynamischer Zeitraum ergibt sich aus dem, was die CSV hergibt
(kein fester Zeitraum).

## Acceptance criteria

- [ ] Adapter liest die echte CSV-CAMT-v8-Beispieldatei ein und erzeugt für
      jede Zeile eine gültige `Buchung`.
- [ ] `Auftragskonto` wird über die Eigene-Konten-Config zu `KONTO_x`
      aufgelöst; Gegen-IBAN wird korrekt zu `Gegenkonto::Eigen` (mit Rolle)
      oder `Gegenkonto::Fremd`.
- [ ] Privatpersonen im Empfänger-Feld werden zu `PERSON_x` pseudonymisiert;
      dieselbe Person erhält innerhalb eines Laufs durchgängig dasselbe
      Pseudonym (Test: zwei Zeilen mit demselben Klartextnamen ergeben
      dasselbe `PERSON_x`).
- [ ] Empfänger/Verwendungszweck landen als `ObfuskierterText` in der
      `Buchung`.
- [ ] **Test**: kein Objekt aus dem Adapter-Output (weder `Buchung` noch eine
      daraus abgeleitete `KategorieAnfrage`) enthält eine rohe IBAN, BIC oder
      Kontonummer — geprüft am realen Beispieldatensatz, nicht nur an
      synthetischen Fixtures.
- [ ] Eine gemischte-Währungen-Zeile (falls in der Datei vorhanden) führt zu
      einem klaren Fehler statt stillschweigender Falschsummierung.
- [ ] `cargo test` deckt den vollständigen Pfad CSV-Datei → `Buchung`-Liste ab
      (Black-Box gegen den DTO-Vertrag, nicht gegen Adapter-Interna).

## Blocked by

- Blocked by [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blocked by [0002](0002-eigene-konten-config-schema-und-loader.md)
- Zusätzliche Voraussetzung (kein Issue): echte CSV-CAMT-v8-Beispieldatei von
  Markus.
