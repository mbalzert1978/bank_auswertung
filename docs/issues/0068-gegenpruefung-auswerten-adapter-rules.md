---
id: "0068"
title: "Gegenprüfung `auswerten_adapter` gegen die `.rules`"
status: open
---

# Gegenprüfung `auswerten_adapter` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#67](https://github.com/mbalzert1978/bank_auswertung/issues/67)

## What to build

Gegenprüfung des Codes von `crates/auswerten_adapter` gegen die nach `.rules/` ausgelagerten
Regeln. **Reines Audit** — je einschlägiger Regel ein Befund; **kein Code wird geändert**.

Einschlägige Regeln:

- `.rules/common/privacy-invarianten.md` — **`kein Serialize` auf dem Kategorisierer-Egress-DTO**
  (`assert_not_impl_any!`); jeder Port sein schmales DTO (Schlüssel = **Fingerabdruck, nie
  Text**); der Seed erreicht kein Egress-DTO; strukturelle Identifikatoren verlassen die
  Maschine nie.
- `.rules/common/architektur.md` — zwei single-purpose Handler (per-window
  `WindowEvaluationHandler` / window-spanning `HistoryEvaluationHandler`, unterschiedliche
  Dispatch-Keys); durchweg terminal; **kein Publish-Seam** (Pipeline-Tail bis `report`);
  read/store/query fehlbar.
- `.rules/crates/dependencies-und-kapselung.md` — kein I/O-Crate; nur `_adapter` bindet
  `application_kernel`; Fremd-Slice nur über dessen `_adapter`.
- `.rules/rust/idiome.md` — Leak = Compile-Fehler; Newtypes.
- `.rules/common/domaenenmodell.md`, `.rules/rust/performance.md`, `.rules/common/sprache.md`,
  `.rules/rust/gates.md`.

## Acceptance criteria

- [ ] Jede gelistete Regel gegen den Crate-Code geprüft; je Regel ein Befund (konform / `file:line`).
- [ ] `Serialize`-Guard auf der Kategorisierer-Anfrage nachgewiesen; Schlüssel = Fingerabdruck, kein Text/Seed.
- [ ] Belegt: kein Publish-Seam; die zwei Handler auf getrennten Dispatch-Keys.
- [ ] Dependency-Richtung bestätigt (kein I/O-Crate; `application_kernel` nur hier).
- [ ] Redigierter Prüf-Report abgelegt; Abweichungen als Folge-Ticket-Vorschlag notiert — **kein Code geändert**.

## Blocked by

- None — kann sofort starten (unabhängiges Audit-Ticket).
