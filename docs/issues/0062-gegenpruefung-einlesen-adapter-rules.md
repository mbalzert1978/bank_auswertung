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
Regeln. **Reines Audit** — je einschlägiger Regel ein Befund; **kein Code wird geändert**.

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

- [ ] Jede gelistete Regel gegen den Crate-Code geprüft; je Regel ein Befund (konform / `file:line`).
- [ ] Presence-Enforcement als redigiert + batch-terminal belegt (`RawLine::try_from` im `SourceBridge`).
- [ ] Dependency-Richtung bestätigt: kein I/O-Crate im `Cargo.toml`, `application_kernel` nur hier gebunden.
- [ ] Status des Egress-`Serialize`-Guards dokumentiert (vorhanden / bekannt offen).
- [ ] Redigierter Prüf-Report abgelegt; Abweichungen als Folge-Ticket-Vorschlag notiert — **kein Code geändert**.

## Blocked by

- None — kann sofort starten (unabhängiges Audit-Ticket).
