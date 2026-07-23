---
id: "0060"
title: "Gegenprüfung `application_kernel` gegen die `.rules`"
status: closed
---

# Gegenprüfung `application_kernel` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#59](https://github.com/mbalzert1978/bank_auswertung/issues/59)

## What to build

Gegenprüfung des Codes von `crates/application_kernel` gegen die frisch nach `.rules/`
ausgelagerten Regeln (Format: kurze Aussage + Do/Don't). **Umsetzungs-Ticket** — der Code von
`crates/application_kernel` wird gegen jede unten gelistete einschlägige `.rules`-Regel
geprüft und, **wo eine Regel verletzt ist, im Code angepasst**, bis die Crate ihren `.rules`
entspricht. Ist die Crate bereits konform, bleibt sie unverändert — **keine künstliche
Änderung, kein neues Ticket, kein Audit-Report**. Redigiert bleibt Pflicht: keine rohen
Identifikatoren/Klartexte in Commit/PR.

Einschlägige Regeln (nur die zutreffenden — der Kernel ist ein interface-only Vertrag, kein
I/O, keine DB, keine Domänen-Rechnung):

- `.rules/crates/dependencies-und-kapselung.md` — „Nur `_adapter` bindet `application_kernel`";
  der Kernel selbst muss **dependency-frei** sein (einziger Slice↔`io`-Vertrag).
- `.rules/rust/idiome.md` — interface-only, kein externes Crate.
- `.rules/common/architektur.md` — die Kontrakte (`Command`/`Handler`/`Publisher`/
  `PublishCommand`/`CommandDecoder`) tragen die Outbox-Kopplung als **Vertrag**, nicht als
  Mechanik; keine Slice-/`io`-Details sickern durch.
- `.rules/common/sprache.md` — Doc-Kommentare Deutsch, Mechanik-Bezeichner Englisch.
- `.rules/rust/gates.md` — `[lints] workspace = true` im `Cargo.toml` gesetzt.

## Acceptance criteria

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] `Cargo.toml` ohne Fremd-Dependency (dependency-frei); `[lints] workspace = true` gesetzt.
- [ ] Keine Slice-/`io`-internen Typen im Kernel.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
