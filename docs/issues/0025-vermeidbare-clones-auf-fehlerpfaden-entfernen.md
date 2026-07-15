---
id: "0025"
title: Vermeidbare Clones auf Fehlerpfaden entfernen
status: closed
---

# Vermeidbare Clones auf Fehlerpfaden entfernen

**Typ:** AFK

> Idiomatik-Review-Befund (Schwere: Niedrig, wiederkehrend). An zwei Stellen wird eine
> ID unbedingt geklont, obwohl der jeweilige Fehler-/Fremd-Pfad sie verwirft oder der
> Wert bereits im Besitz der Funktion ist.

## Was zu bauen ist

Zwei Ownership-Straffungen im `obfuskierung`-Slice, ohne Verhaltensänderung:

- **`obfuskierung_adapter` (SourceBridge):** die Projektion einer Roh-Zeile in eine
  Domänen-Buchung klont die `BookingId` **unbedingt**, obwohl der `Malformed`-Pfad sie
  verwirft und nur der `Ok`-Pfad sie an das Aggregat weiterreicht. Die Konvertierung soll
  die ID **borrowend** entgegennehmen und nur im `Ok`-Zweig klonen, sodass der (seltene)
  Fehlerpfad nichts alloziert.
- **`obfuskierung_domain` (Handler):** `handle` nimmt das Command **by value**
  (`handle(&self, command: ObfuscateBooking)`) und klont die `BookingId` im „Absent"-Arm
  (`HandleError::SourceMissing(command.id().clone())`). **Entschieden:** die by-value-Signatur
  bleiben lassen und einen konsumierenden `into_id(self) -> BookingId` auf `ObfuscateBooking`
  ergänzen; der „Absent"-Arm nutzt `command.into_id()` statt `command.id().clone()` und
  **eliminiert den Klon** auf dem Fehlerpfad (nicht bloß verschieben). *Borrowend nehmen ist
  hier bewusst verworfen:* aus einem `&command` müsste der Fehler die ID weiterhin klonen —
  das Ziel des Issues (Klon auf dem Fehlerpfad weg) wäre verfehlt. Die Aufrufstelle im
  `obfuskierung_adapter`-Handler übergibt bereits ein eigenes Temporary
  (`.handle(ObfuscateBooking::new(...))`) und bleibt **unverändert**.

## Abnahmekriterien

- [ ] Die SourceBridge-Konvertierung nimmt die `BookingId` borrowend und klont nur im
      `Ok`-Pfad; der `Malformed`-Fehlerpfad alloziert keine ID mehr.
- [ ] `ObfuscateBooking` hat ein konsumierendes `into_id(self) -> BookingId`; der
      `obfuskierung_domain`-Handler nutzt es im „Absent"-Arm statt `id().clone()` und klont
      die `BookingId` auf dem Fehlerpfad nicht mehr. Die `handle`-by-value-Signatur und die
      Adapter-Aufrufstelle bleiben unverändert.
- [ ] Verhalten unverändert: die `obfuskierung`-Slice-Specs (inkl. absent-Row-Retry und
      Redaktion) bleiben grün; nur `obfuskierung_*`-Crates berührt.
- [ ] `cargo build`/`test`/`clippy --all-targets`/`fmt --check` und
      `cargo test -p architecture` grün.

## Blockiert durch

- None - can start immediately.

**Reihenfolge (kein Blocker, aber Datei-Überlappung):** fasst `source_bridge.rs` /
`ingested_row.rs` an, die auch
[0024](0024-dto-domaene-mapping-auf-from-tryfrom-umstellen-tryfrom-wenn-fallibel-sonst-from.md)
berührt. Den `obfuskierung`-Strang **0024 → 0025 → 0026 seriell** fahren; nach 0024 laufen,
davor 0026. Parallel dazu sind
[0023](0023-error-ketten-vervollstaendigen-source-an-wrapper-errors.md) und
[0027](0027-spec-testcode-aufraeumen-let-else-vec-wiederholung-fixture-helfer.md) disjunkt.
