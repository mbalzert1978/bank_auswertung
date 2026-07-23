---
id: "0061"
title: "Gegenprüfung `einlesen_domain` gegen die `.rules`"
status: closed
---

# Gegenprüfung `einlesen_domain` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#60](https://github.com/mbalzert1978/bank_auswertung/issues/60)

## What to build

Gegenprüfung des Codes von `crates/einlesen_domain` gegen die nach `.rules/` ausgelagerten
Regeln. **Umsetzungs-Ticket** — der Code von `crates/einlesen_domain` wird gegen jede unten
gelistete einschlägige `.rules`-Regel geprüft und, **wo eine Regel verletzt ist, im Code
angepasst**, bis die Crate ihren `.rules` entspricht. Ist die Crate bereits konform, bleibt
sie unverändert — **keine künstliche Änderung, kein neues Ticket, kein Audit-Report**.
Redigiert bleibt Pflicht: keine rohen Identifikatoren/Klartexte in Commit/PR.

Einschlägige Regeln (dependency-freie Domäne — SQL-/Nebenläufigkeits-/Serialize-Guard-Regeln
greifen hier bewusst nicht):

- `.rules/common/domaenenmodell.md` — Value Objects statt Primitiven, Summentypen
  (`Direction`/`Currency`/`CalendarDay`/`Sign`), `Betrag` vorzeichenlos (Vorzeichen in
  `Richtung`), deklarativ-funktional.
- `.rules/rust/idiome.md` — `_domain` **dependency-frei** (kein `serde`/`thiserror`);
  Domänen-Fehler als handgeschriebenes `Display` + `std::error::Error`; Newtypes validieren
  beim Konstruieren (`RawLine`-Pflichtfelder non-`Option`, illegale States unkonstruierbar).
- `.rules/common/architektur.md` — externe Information nur hinter Port (`RawLineSource`,
  `Calendar`/`Money`); durchweg terminal (nackter `LineError`); Aggregate referenzieren
  Fremd-Entitäten nur per ID.
- `.rules/crates/dependencies-und-kapselung.md` — kein Fremd-Crate, kein `application_kernel`,
  kein Griff in ein fremdes `_domain`.
- `.rules/rust/performance.md` — Zero-Cost-Iteratoren, keine unnötigen `collect`.
- `.rules/rust/testing-werkzeuge.md` — `proptest` für Aggregat-Invarianten (soweit vorhanden/sinnvoll).
- `.rules/common/sprache.md`, `.rules/rust/gates.md`.

## Acceptance criteria

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] Dependency-Freiheit (kein `serde`/`thiserror`/I/O-Crate im `Cargo.toml`); fehlte sie, hergestellt.
- [ ] Pflichtfelder als Typ (non-`Option`) statt per Laufzeit-Check getragen; sonst umgestellt.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
