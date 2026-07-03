# Issue-Fortschritt

Stand: 2026-07-03

## Neu (offen) — event-getriebene Pipeline (ADR-0008)

Aus der Fortschreibung der PRD nach [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md)
(event-getriebene Pipeline aus Slices mit Outbox) abgeleitet — deckt die bislang
unbelegten Messaging-/Pipeline-Teile ab. Die Fachfunktions-Issues (0004–0013)
bleiben gültig; nur ihre Verortung wandert in die Pipeline-Slices.

- [ ] **[0014-shared-kernel-outbox-vertraege.md](0014-shared-kernel-outbox-vertraege.md)**
  `shared_kernel` um die Outbox-Verträge erweitern (interface-only, nur `std`).
- [ ] **[0015-einlesen-slice-als-pipeline-consumer.md](0015-einlesen-slice-als-pipeline-consumer.md)**
  `einlesen`-Slice als Pipeline-Consumer (Umbau von `csv_camt_v8`), Message `Eingelesen`.
- [ ] **[0016-obfuskieren-slice-als-pipeline-consumer.md](0016-obfuskieren-slice-als-pipeline-consumer.md)**
  `obfuskieren`-Slice als Consumer von `Eingelesen`, Message `Obfuskiert` (Datenschutz-Grenze).

Vertagt (Zielbild ADR-0008, bewusst noch kein Issue): `auswerten`-Consumer,
`report`-Consumer, `io`-Outbox-Broker (SQLite) — die Fachlogik ist bereits durch
0004–0007 / 0009 / 0012 abgedeckt.

## Erledigt

- [x] **[0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)** (2026-07-01)
  Domänen-Skelett: Buchung, Fingerabdruck, ObfuskierterText, KategorieAnfrage
- [x] **[0003-csv-camt-v8-adapter-zu-buchung.md](0003-csv-camt-v8-adapter-zu-buchung.md)** (2026-07-01)
  CSV-CAMT-v8-Adapter → `Buchung` (Umsatzquelle-Port), obfuskiert beim Mappen;
  getestet gegen synthetische Fixtures nach echtem Header. Echtdaten-Verifikation
  vertagt (Realdatei via CLI-Argument, sobald das Programm vollständig ist).
