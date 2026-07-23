---
id: "0064"
title: "Gegenprüfung `obfuskierung_adapter` gegen die `.rules`"
status: open
---

# Gegenprüfung `obfuskierung_adapter` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#63](https://github.com/mbalzert1978/bank_auswertung/issues/63)

## What to build

Gegenprüfung des Codes von `crates/obfuskierung_adapter` gegen die nach `.rules/`
ausgelagerten Regeln. **Reines Audit** — je einschlägiger Regel ein Befund; **kein Code wird
geändert**.

Einschlägige Regeln:

- `.rules/common/privacy-invarianten.md` — **`kein Serialize` auf dem Egress-DTO**
  (`ObfuscatedBooking` trägt den `assert_not_impl_any!`-Guard); strukturelle Identifikatoren
  verlassen die Maschine nie; **Fingerabdruck gehört nicht in den Adapter** (Do/Don't:
  Domänenwissen nicht im Adapter); jeder Port sein schmales DTO.
- `.rules/common/architektur.md` — Outbox-Kopplung (liest per ID, schreibt eigene Tabelle,
  publiziert die **neue** ID als `Obfuscated`-Zeiger); durchweg terminal; uniformes ID-Handoff.
- `.rules/crates/dependencies-und-kapselung.md` — kein I/O-Crate; nur `_adapter` bindet
  `application_kernel`; kein Griff ins fremde `_domain`.
- `.rules/rust/idiome.md` — Leak = Compile-Fehler (`static_assertions`); `IngestedRow`-DTO als
  Newtype-Grenze.
- `.rules/common/domaenenmodell.md`, `.rules/rust/performance.md`, `.rules/common/sprache.md`,
  `.rules/rust/gates.md`.

## Acceptance criteria

- [ ] Jede gelistete Regel gegen den Crate-Code geprüft; je Regel ein Befund (konform / `file:line`).
- [ ] `assert_not_impl_any!(ObfuscatedBooking: Serialize)`-Guard nachgewiesen.
- [ ] Belegt: kein Fingerabdruck-/Domänenwissen im Adapter; kein roher Identifikator im Egress.
- [ ] ID-Handoff belegt: liest per ID, publiziert die neue Store-ID als Zeiger.
- [ ] Redigierter Prüf-Report abgelegt; Abweichungen als Folge-Ticket-Vorschlag notiert — **kein Code geändert**.

## Blocked by

- None — kann sofort starten (unabhängiges Audit-Ticket).
