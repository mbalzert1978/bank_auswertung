---
id: "0064"
title: "Gegenprüfung `obfuskierung_adapter` gegen die `.rules`"
status: closed
---

# Gegenprüfung `obfuskierung_adapter` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#63](https://github.com/mbalzert1978/bank_auswertung/issues/63)

## What to build

Gegenprüfung des Codes von `crates/obfuskierung_adapter` gegen die nach `.rules/`
ausgelagerten Regeln. **Umsetzungs-Ticket** — der Code von `crates/obfuskierung_adapter` wird
gegen jede unten gelistete einschlägige `.rules`-Regel geprüft und, **wo eine Regel verletzt
ist, im Code angepasst**, bis die Crate ihren `.rules` entspricht. Ist die Crate bereits
konform, bleibt sie unverändert — **keine künstliche Änderung, kein neues Ticket, kein
Audit-Report**. Redigiert bleibt Pflicht: keine rohen Identifikatoren/Klartexte in Commit/PR.

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

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] `assert_not_impl_any!(ObfuscatedBooking: Serialize)`-Guard vorhanden; fehlte er, ergänzt.
- [ ] Kein Fingerabdruck-/Domänenwissen im Adapter; kein roher Identifikator im Egress — sonst bereinigt.
- [ ] ID-Handoff: liest per ID, publiziert die neue Store-ID als Zeiger; sonst hergestellt.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
