---
id: "0060"
title: "Gegenprüfung `application_kernel` gegen die `.rules`"
status: open
---

# Gegenprüfung `application_kernel` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#59](https://github.com/mbalzert1978/bank_auswertung/issues/59)

## What to build

Gegenprüfung des Codes von `crates/application_kernel` gegen die frisch nach `.rules/`
ausgelagerten Regeln (Format: kurze Aussage + Do/Don't). **Reines Audit** — der Crate-Code
wird je einschlägiger Regel geprüft; **kein Code wird in diesem Ticket geändert**.
Abweichungen werden redigiert festgehalten und als Folge-Ticket-Vorschläge geschnitten.

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

- [ ] Jede gelistete Regel gegen den Crate-Code geprüft; je Regel ein Befund (konform /
      Abweichung mit `file:line`).
- [ ] `Cargo.toml` ohne Fremd-Dependency verifiziert (dependency-frei); `[lints] workspace = true` bestätigt.
- [ ] Bestätigt, dass keine Slice-/`io`-internen Typen im Kernel auftauchen.
- [ ] Ergebnis als kurzer, redigierter Prüf-Report abgelegt; Abweichungen als Folge-Ticket-Vorschlag notiert — **kein Code geändert**.

## Blocked by

- None — kann sofort starten (unabhängiges Audit-Ticket).
