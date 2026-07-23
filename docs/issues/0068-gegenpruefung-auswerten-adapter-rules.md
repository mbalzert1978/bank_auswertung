---
id: "0068"
title: "Gegenprüfung `auswerten_adapter` gegen die `.rules`"
status: closed
---

# Gegenprüfung `auswerten_adapter` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#67](https://github.com/mbalzert1978/bank_auswertung/issues/67)

## What to build

Gegenprüfung des Codes von `crates/auswerten_adapter` gegen die nach `.rules/` ausgelagerten
Regeln. **Umsetzungs-Ticket** — der Code von `crates/auswerten_adapter` wird gegen jede unten
gelistete einschlägige `.rules`-Regel geprüft und, **wo eine Regel verletzt ist, im Code
angepasst**, bis die Crate ihren `.rules` entspricht. Ist die Crate bereits konform, bleibt
sie unverändert — **keine künstliche Änderung, kein neues Ticket, kein Audit-Report**.
Redigiert bleibt Pflicht: keine rohen Identifikatoren/Klartexte in Commit/PR.

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

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] `Serialize`-Guard auf der Kategorisierer-Anfrage vorhanden; Schlüssel = Fingerabdruck, kein Text/Seed — sonst hergestellt.
- [ ] Kein Publish-Seam; die zwei Handler auf getrennten Dispatch-Keys — sonst hergestellt.
- [ ] Dependency-Richtung eingehalten (kein I/O-Crate; `application_kernel` nur hier).
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
