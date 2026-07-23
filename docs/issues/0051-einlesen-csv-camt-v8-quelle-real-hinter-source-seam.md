---
id: "0051"
title: "einlesen: CSV-CAMT-v8-Quelle real hinter dem Source-Seam"
status: closed
---

# einlesen: CSV-CAMT-v8-Quelle real hinter dem Source-Seam

**Typ:** AFK

## What to build

Die echte Erfüllung des `SourceSeam` (Domänen-Port `RawLineSource`) in `io`: eine CSV-CAMT-v8-Datei
über `fs` lesen, ihre Zeilen zu `RawRecord`s parsen und **in-process** über den Source-Seam an den
einlesen-Slice reichen (seit [0018](0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
fetcht der Handler den Batch über den Port, nicht per Command). Header nach Standard-Sparkassen-Layout
([docs/examples/csv_camt_v8_header.md](../examples/csv_camt_v8_header.md)).

- Der Adapter sichert **eine Währung über die ganze Datei** zu — die Domäne summiert keine gemischten
  Währungen; ein Währungsbruch schlägt terminal (redigiert) fehl.
- **Präsenz-Prüfung bleibt der Adapter-ACL** (`RawLine::try_from`, seit
  [0018](0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)): ein fehlendes
  Pflichtfeld mappt auf `SourceError::MissingMandatoryField` (nur statischer Feldname) und failt den
  Batch terminal.
- **Roh-IBAN/BIC bleiben io-lokal**, kein Egress ([ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md)).

**Keine Realdatei nötig.** Gebaut und getestet gegen **synthetische Fixtures** nach echtem Header — die
Verifikation gegen Echtdaten bleibt bewusst **vertagt**: die Realdatei liefert Markus per
CLI-Argument, wenn das Programm vollständig steht (Prämisse aus
[0003](0003-csv-camt-v8-adapter-zu-buchung.md), historischer `csv_camt_v8`-Stand). Persistenzfrei —
hängt **nicht** an der Stack-Entscheidung.

## Acceptance criteria

- [ ] Der `SourceSeam` ist in `io` durch einen echten CSV-CAMT-v8-Reader (fs + csv) erfüllt; eine
      Fixture-Datei ergibt die erwarteten `RawRecord`s.
- [ ] Einwährungs-Zusicherung über die Datei; ein Währungsbruch **und** ein fehlendes Pflichtfeld
      schlagen je terminal + redigiert fehl (nie ein Rohwert im Fehler).
- [ ] Downstream gegen Fakes verifizierbar (Fixture → `RawRecord`s → `RawLine`s); **kein** echtes
      SQLite nötig.
- [ ] csv-/fs-Deps liegen **nur in `io`**; `cargo test -p architecture` bleibt grün.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`,
      `cargo test -p architecture` und `cargo doc --no-deps --workspace` sind grün.

## Blocked by

None - can start immediately. (Persistenzfrei; hängt **nicht** an
[0049](0049-persistenz-stack-rusqlite-synchron-kein-sqlx.md).)
