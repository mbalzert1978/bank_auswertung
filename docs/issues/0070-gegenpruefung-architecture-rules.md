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
gegen die nach `.rules/` ausgelagerten Regeln. **Reines Audit** — je einschlägiger Regel ein
Befund; **kein Code wird geändert**. Diese Crate **erzwingt** selbst die Dependency-Richtung —
geprüft wird also, ob der Test die in `.rules/` kodifizierten Regeln vollständig abbildet.

Einschlägige Regeln:

- `.rules/crates/dependencies-und-kapselung.md` — der `architecture`-Test (über
  `cargo_metadata`) muss alle Kapselungs-Regeln abdecken: nur `io` bindet Fremd-Crates; nur
  `_adapter` bindet `application_kernel`; Domäne dependency-frei; kein `_domain`↔`_domain`.
  Lücke zwischen Regeltext und erzwungenem Test = Befund.
- `.rules/rust/gates.md` — `cargo test -p architecture` ist eines der sechs Gates.
- `.rules/common/sprache.md` — Testnamen Deutsch.
- `.rules/rust/testing-werkzeuge.md` — `proptest`, falls einschlägig.

## Acceptance criteria

- [ ] Jede gelistete Regel gegen den Crate-Code geprüft; je Regel ein Befund (konform / `file:line`).
- [ ] Jede Kapselungs-Regel aus `dependencies-und-kapselung.md` einer Assertion im Test zugeordnet; nicht abgedeckte Regeln als Lücke benannt.
- [ ] Bestätigt, dass `cargo test -p architecture` als Gate greift und die Testnamen der Sprachkonvention folgen.
- [ ] Redigierter Prüf-Report abgelegt; Abweichungen/Lücken als Folge-Ticket-Vorschlag notiert — **kein Code geändert**.

## Blocked by

- None — kann sofort starten (unabhängiges Audit-Ticket).
