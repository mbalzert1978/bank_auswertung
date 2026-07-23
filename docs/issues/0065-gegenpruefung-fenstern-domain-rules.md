---
id: "0065"
title: "Gegenprüfung `fenstern_domain` gegen die `.rules`"
status: open
---

# Gegenprüfung `fenstern_domain` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#64](https://github.com/mbalzert1978/bank_auswertung/issues/64)

## What to build

Gegenprüfung des Codes von `crates/fenstern_domain` gegen die nach `.rules/` ausgelagerten
Regeln. **Reines Audit** — je einschlägiger Regel ein Befund; **kein Code wird geändert**.

Einschlägige Regeln:

- `.rules/common/domaenenmodell.md` — `Auswertungsfenster`-Invarianten bei Konstruktion
  (**non-empty** + **eine Währung**); Summentypen; deklarativ (N→1-Reduktion per
  `fold`/Iterator statt mutabler Schleife).
- `.rules/common/architektur.md` — `WindowAnchor`-Port (data-relativ, **keine Uhr** in der
  Domäne); durchweg terminal; uniformes ID-Handoff (Seite per ID lesen, neue ID als
  `FensterGeschlossen` publizieren).
- `.rules/rust/idiome.md` — `_domain` dependency-frei; Newtypes.
- `.rules/rust/performance.md` — Zero-Cost-Iteratoren in der Reduktion.
- `.rules/rust/testing-werkzeuge.md` — `proptest` für die Fenster-Invarianten.
- `.rules/crates/dependencies-und-kapselung.md`, `.rules/common/sprache.md`, `.rules/rust/gates.md`.

## Acceptance criteria

- [ ] Jede gelistete Regel gegen den Crate-Code geprüft; je Regel ein Befund (konform / `file:line`).
- [ ] Fenster-Invarianten (non-empty + eine Währung) als konstruktions-erzwungen belegt.
- [ ] Belegt: keine Kalender-/Uhr-Bindung in der Domäne; Tagesstempel-Achse kommt als Wert herein.
- [ ] Dependency-Freiheit der Domäne verifiziert.
- [ ] Redigierter Prüf-Report abgelegt; Abweichungen als Folge-Ticket-Vorschlag notiert — **kein Code geändert**.

## Blocked by

- None — kann sofort starten (unabhängiges Audit-Ticket).
