---
id: "0066"
title: "Gegenprüfung `fenstern_adapter` gegen die `.rules`"
status: open
---

# Gegenprüfung `fenstern_adapter` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#65](https://github.com/mbalzert1978/bank_auswertung/issues/65)

## What to build

Gegenprüfung des Codes von `crates/fenstern_adapter` gegen die nach `.rules/` ausgelagerten
Regeln. **Reines Audit** — je einschlägiger Regel ein Befund; **kein Code wird geändert**.

Einschlägige Regeln:

- `.rules/common/architektur.md` — zwei single-purpose Handler (Start/Windowing);
  Outbox-Kopplung (`AdvanceWindow`/`FensterGeschlossen`/`HistorieVollständig`); die
  **Selbst-Fortsetzung** (nächster Anker per `max(booking_day < from)`) läuft über die Outbox
  statt Folgeaufruf; durchweg terminal; uniformes ID-Handoff.
- `.rules/crates/dependencies-und-kapselung.md` — kein I/O-Crate; nur `_adapter` bindet
  `application_kernel`; Fremd-Slice nur über dessen `_adapter`.
- `.rules/common/privacy-invarianten.md` — der Slice sieht nur Tokens/opake IDs/Tagesstempel
  und hat **kein Egress-DTO**, daher **kein** `Serialize`-Guard erwartet — prüfen, dass das so
  bleibt und nichts Rohes auftaucht.
- `.rules/common/domaenenmodell.md`, `.rules/rust/idiome.md`, `.rules/rust/performance.md`,
  `.rules/common/sprache.md`, `.rules/rust/gates.md`.

## Acceptance criteria

- [ ] Jede gelistete Regel gegen den Crate-Code geprüft; je Regel ein Befund (konform / `file:line`).
- [ ] Belegt: die zwei Handler koppeln nur über die Outbox; „leerer Store = Ok, publiziert nichts".
- [ ] Belegt: kein roher Identifikator, kein Egress-DTO (und damit keine fehlende Guard-Pflicht).
- [ ] Dependency-Richtung bestätigt (kein I/O-Crate; `application_kernel` nur hier).
- [ ] Redigierter Prüf-Report abgelegt; Abweichungen als Folge-Ticket-Vorschlag notiert — **kein Code geändert**.

## Blocked by

- None — kann sofort starten (unabhängiges Audit-Ticket).
