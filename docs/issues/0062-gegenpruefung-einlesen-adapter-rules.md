---
id: "0062"
title: "Gegenprüfung `einlesen_adapter` gegen die `.rules`"
status: open
---

# Gegenprüfung `einlesen_adapter` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#61](https://github.com/mbalzert1978/bank_auswertung/issues/61)

## What to build

Gegenprüfung des Codes von `crates/einlesen_adapter` gegen die nach `.rules/` ausgelagerten
Regeln. **Umsetzungs-Ticket** — der Code von `crates/einlesen_adapter` wird gegen jede unten
gelistete einschlägige `.rules`-Regel geprüft und, **wo eine Regel verletzt ist, im Code
angepasst**, bis die Crate ihren `.rules` entspricht. Ist die Crate bereits konform, bleibt
sie unverändert — **keine künstliche Änderung, kein neues Ticket, kein Audit-Report**.
Redigiert bleibt Pflicht: keine rohen Identifikatoren/Klartexte in Commit/PR.

Einschlägige Regeln:

- `.rules/common/architektur.md` — Presence-Enforcement im ACL (`SourceBridge`,
  `RawLine::try_from`); durchweg terminal (redigierter `SourceError::MissingMandatoryField`,
  Batch-atomar); Outbox-Kopplung (ein `Ingested` pro gültiger Zeile statt Folgeaufruf);
  uniformes ID-Handoff.
- `.rules/common/privacy-invarianten.md` — redigierte Fehler (statischer Feldname statt
  Rohwert). **Hinweis:** der `Serialize`-Guard auf der `Line`-Egress-Projektion ist laut
  CLAUDE.md **noch deferred** — hier prüfen, ob er fehlt, und den Status als Befund notieren
  (erwartete offene Baustelle, kein Verstoß); zusätzlich bestätigen, dass die Projektion
  faktisch keinen rohen Identifikator ausleitet.
- `.rules/crates/dependencies-und-kapselung.md` — kein I/O-Crate (`csv` gehört in `io`); nur
  `_adapter` bindet `application_kernel`; Zugriff auf fremde Slices nur über deren `_adapter`;
  `serde`/`thiserror`/`static_assertions` als Ausnahme erlaubt.
- `.rules/common/domaenenmodell.md` — die adapter-eigenen Summentyp-DTOs
  (`Direction`/`Currency`/…) statt re-exportierter Domänentypen.
- `.rules/rust/idiome.md` — Newtypes; Leak-Garantie über fehlendes `Serialize`.
- `.rules/rust/performance.md`, `.rules/common/sprache.md`, `.rules/rust/gates.md`.

## Acceptance criteria

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] Presence-Enforcement ist redigiert + batch-terminal (`RawLine::try_from` im `SourceBridge`); sonst hergestellt.
- [ ] Dependency-Richtung eingehalten: kein I/O-Crate im `Cargo.toml`, `application_kernel` nur hier gebunden.
- [ ] Egress-`Serialize`-Guard: der `Line`-Guard ist laut CLAUDE.md bekannt offen — bleibt vertagt, kein künstlicher Nachbau; die Projektion leitet faktisch keinen rohen Identifikator aus.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
