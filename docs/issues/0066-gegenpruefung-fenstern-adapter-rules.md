---
id: "0066"
title: "Gegenprüfung `fenstern_adapter` gegen die `.rules`"
status: closed
---

# Gegenprüfung `fenstern_adapter` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#65](https://github.com/mbalzert1978/bank_auswertung/issues/65)

## What to build

Gegenprüfung des Codes von `crates/fenstern_adapter` gegen die nach `.rules/` ausgelagerten
Regeln. **Umsetzungs-Ticket** — der Code von `crates/fenstern_adapter` wird gegen jede unten
gelistete einschlägige `.rules`-Regel geprüft und, **wo eine Regel verletzt ist, im Code
angepasst**, bis die Crate ihren `.rules` entspricht. Ist die Crate bereits konform, bleibt
sie unverändert — **keine künstliche Änderung, kein neues Ticket, kein Audit-Report**.
Redigiert bleibt Pflicht: keine rohen Identifikatoren/Klartexte in Commit/PR.

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

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] Die zwei Handler koppeln nur über die Outbox; „leerer Store = Ok, publiziert nichts" — sonst hergestellt.
- [ ] Kein roher Identifikator, kein Egress-DTO (und damit keine Guard-Pflicht); bleibt so.
- [ ] Dependency-Richtung eingehalten (kein I/O-Crate; `application_kernel` nur hier).
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
