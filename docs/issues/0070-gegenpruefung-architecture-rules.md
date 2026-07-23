---
id: "0070"
title: "Gegenprüfung `architecture` gegen die `.rules`"
status: open
---

# Gegenprüfung `architecture` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#69](https://github.com/mbalzert1978/bank_auswertung/issues/69)

## What to build

Gegenprüfung des Codes der Top-Level-Crate `architecture` (Test-only, außerhalb `crates/`)
gegen die nach `.rules/` ausgelagerten Regeln. **Umsetzungs-Ticket** — der Code von
`architecture` wird gegen jede unten gelistete einschlägige `.rules`-Regel geprüft und, **wo
eine Regel verletzt ist, im Code angepasst**, bis die Crate ihren `.rules` entspricht. Ist die
Crate bereits konform, bleibt sie unverändert — **keine künstliche Änderung, kein neues
Ticket, kein Audit-Report**. Redigiert bleibt Pflicht: keine rohen Identifikatoren/Klartexte
in Commit/PR. Diese Crate **erzwingt** selbst die Dependency-Richtung — geprüft (und wo nötig
ergänzt) wird also, ob der Test die in `.rules/` kodifizierten Regeln vollständig abbildet.

Einschlägige Regeln:

- `.rules/crates/dependencies-und-kapselung.md` — der `architecture`-Test (über
  `cargo_metadata`) muss alle Kapselungs-Regeln abdecken: nur `io` bindet Fremd-Crates; nur
  `_adapter` bindet `application_kernel`; Domäne dependency-frei; kein `_domain`↔`_domain`.
  Lücke zwischen Regeltext und erzwungenem Test = Befund.
- `.rules/rust/gates.md` — `cargo test -p architecture` ist eines der sechs Gates.
- `.rules/common/sprache.md` — Testnamen Deutsch.
- `.rules/rust/testing-werkzeuge.md` — `proptest`, falls einschlägig.

## Acceptance criteria

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] Jede Kapselungs-Regel aus `dependencies-und-kapselung.md` ist einer Assertion im Test zugeordnet; nicht abgedeckte Regeln als fehlende Assertion ergänzt.
- [ ] `cargo test -p architecture` greift als Gate und die Testnamen folgen der Sprachkonvention; sonst hergestellt.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
