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
Regeln. **Umsetzungs-Ticket** — der Code von `crates/fenstern_domain` wird gegen jede unten
gelistete einschlägige `.rules`-Regel geprüft und, **wo eine Regel verletzt ist, im Code
angepasst**, bis die Crate ihren `.rules` entspricht. Ist die Crate bereits konform, bleibt
sie unverändert — **keine künstliche Änderung, kein neues Ticket, kein Audit-Report**.
Redigiert bleibt Pflicht: keine rohen Identifikatoren/Klartexte in Commit/PR.

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

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] Fenster-Invarianten (non-empty + eine Währung) konstruktions-erzwungen; sonst hergestellt.
- [ ] Keine Kalender-/Uhr-Bindung in der Domäne; Tagesstempel-Achse kommt als Wert herein — sonst umgestellt.
- [ ] Dependency-Freiheit der Domäne; fehlte sie, hergestellt.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
