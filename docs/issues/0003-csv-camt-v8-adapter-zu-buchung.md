---
id: "0003"
title: "CSV-CAMT-v8-Adapter zu Buchung"
status: done
---

# CSV-CAMT-v8-Adapter zu Buchung

> **Umbau durch [ADR-0007](../adr/0007-modularer-monolith-feature-slices.md).** Der
> Adapter wird zur Slice-Triade `csv_camt_v8_domain`/`_adapter`/`_specs`: der public
> Port gibt einen **DTO** heraus (nicht `Buchung`), die Obfuskierung wandert in den
> eigenen `obfuskierung`-Slice (per Bridge konsumiert), die Eigen/Fremd-Auflösung
> läuft über die public API des `eigene_konten`-Slice. Fachlicher Inhalt (Mapping,
> einheitliche Währung, Fail-safe-Empfänger, Datenschutz) bleibt; nur der Schnitt
> und die Testführung (Fluent-TestApi, Black-Box) ändern sich.

**Prämisse:** Dieser Slice ist **nicht** auf eine echte CSV-Beispieldatei
blockiert. Es gibt vorerst keine Realdaten — die liefert Markus erst, wenn das
Programm vollständig steht, und dann ausschließlich per CLI-Argument. Für diesen
Slice wurden die Testdaten daher als **synthetische CSV-CAMT-v8-Fixtures nach dem
echten Header** ([docs/examples/csv_camt_v8_header.md](../examples/csv_camt_v8_header.md),
Standard-Sparkassen-Layout) erzeugt. Die Verifikation gegen Echtdaten ist bewusst
**vertagt** (Programm vollständig → Realdatei via Argument).

## Was zu bauen ist

Der `Umsatzquelle`-Port (inbound), CSV-CAMT-v8-Adapter zuerst
(Baureihenfolge-Schritt 2): liest eine CSV-CAMT-v8-Datei ein und mappt jede Zeile
auf eine `Buchung` — **obfuskiert beim Mappen**, sodass die Domäne nur obfuskierte
Value Objects erhält.

Im Adapter (nicht in der Domäne): `Auftragskonto` → `KONTO_x` (First-Seen, für
diesen Slice **nur laufstabil innerhalb eines Programmlaufs**; das Auftragskonto
muss über die Eigene-Konten-Config als eigenes Konto belegt sein); Gegen-IBAN →
`Gegenkonto::Eigen` (mit Konto-Rolle) | `Fremd` (über die Eigene-Konten-Config);
Privatperson im Empfänger-Feld → `PERSON_x` (First-Seen, nur laufstabil —
laufübergreifende Persistenz kommt mit der persistenten Pseudonym-Zuordnung);
Freitext (Empfänger/Verwendungszweck) → `ObfuskierterText` (leerer
Verwendungszweck → `None`). Rohe IBAN/BIC bleiben ausschließlich im Adapter. Der
Adapter sichert eine einheitliche Währung über die gesamte Datei zu — die Domäne
summiert keine gemischten Währungen.

Ein realer, dynamischer Zeitraum ergibt sich aus dem, was die CSV hergibt (kein
fester Zeitraum).

## Umgesetzt: Modell-Folgeänderung

Damit die Analyse (Schritt 3) die Konto-Rolle einer Umbuchung nach der
Obfuskierung nicht verliert, trägt `Gegenkonto::Eigen` jetzt die `Rolle`
(`Gegenkonto::Eigen(Rolle)`). `Rolle` ist Domänenvokabular und wohnt in der
Domäne; `eigene_konten` re-exportiert es. Analog wird ein fehlender Freitext am
Feld modelliert: `Buchung::verwendungszweck: Option<ObfuskierterText>` (und die
Projektion `KategorieAnfrage` ebenso).

## Abnahmekriterien

- [x] Adapter liest eine CSV-CAMT-v8-Datei ein und erzeugt für jede Zeile eine
      gültige `Buchung` (getestet gegen synthetische Fixtures).
- [x] `Auftragskonto` wird über die Eigene-Konten-Config zu `KONTO_x` aufgelöst;
      Gegen-IBAN wird korrekt zu `Gegenkonto::Eigen` (mit Rolle) oder
      `Gegenkonto::Fremd`.
- [x] Privatpersonen im Empfänger-Feld werden zu `PERSON_x` pseudonymisiert;
      dieselbe Person erhält innerhalb eines Laufs durchgängig dasselbe Pseudonym
      (Test: zwei Zeilen mit demselben Klartextnamen ergeben dasselbe `PERSON_x`).
- [x] Empfänger/Verwendungszweck landen als `ObfuskierterText` in der `Buchung`
      (leerer Verwendungszweck → `None`).
- [x] **Test**: kein Objekt aus dem Adapter-Output (weder `Buchung` noch eine
      daraus abgeleitete `KategorieAnfrage`) enthält eine rohe IBAN, BIC oder
      Kontonummer. _Angepasst:_ geprüft gegen die synthetischen Fixtures; die
      Prüfung am **realen** Beispieldatensatz ist vertagt (siehe Prämisse).
- [x] Eine gemischte-Währungen-Zeile führt zu einem klaren Fehler statt
      stillschweigender Falschsummierung.
- [x] `cargo test` deckt den vollständigen Pfad CSV-Datei → `Buchung`-Liste ab
      (Black-Box gegen den DTO-Vertrag, nicht gegen Adapter-Interna).

## Vertagt (auf spätere Schritte)

- Verifikation gegen einen **realen** CSV-CAMT-v8-Beispieldatensatz (Realdatei
  via CLI-Argument, sobald das Programm vollständig ist).
- Laufübergreifend stabile `PERSON_x` (persistente Pseudonym-Zuordnung, Schritt
  4) und `KONTO_x`.
- Best-effort-Scrubbing eingebetteter Identifikatoren im `ObfuskierterText`
  (Schritt 5).
- **Firmen-Klartextname als Kategorie-Signal.** Das Domänen-Skelett (Schritt 1)
  trägt in `Empfänger::Firma` nur die Gläubiger-ID, nicht den Firmennamen — die
  `KategorieAnfrage` reicht dem Orakel damit die Creditor-ID (`DE98ZZZ…`) statt
  „Netflix". CONTEXT.md sieht dagegen vor, dass der **Firmen**name (anders als
  der pseudonymisierte Personenname) im Klartext als wichtigstes Kategorie-Signal
  erhalten bleibt. Die Modell-Erweiterung (`Empfänger::Firma` führt den
  Firmennamen zusätzlich als `ObfuskierterText`, während der Fingerabdruck weiter
  auf der Gläubiger-ID schlüsselt) berührt das committete Schritt-1-Skelett und
  die Fingerabdruck-Semantik und ist damit bewusst außerhalb dieses
  Adapter-Slices — sie gehört zur Fingerabdruck-/`Kategorisierer`-Arbeit. Der
  Adapter mappt bereits fail-safe korrekt; er verwirft den Firmennamen nur, weil
  das Domänen-Modell (noch) keinen Platz dafür hat.

## Blockiert durch

- ~~Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)~~
  — erledigt.
- ~~Blockiert durch [0002](0002-eigene-konten-config-schema-und-loader.md)~~ —
  Loader + Schema vorhanden und genutzt.
