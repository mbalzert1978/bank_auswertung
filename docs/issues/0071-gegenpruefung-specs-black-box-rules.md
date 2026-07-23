---
id: "0071"
title: "Gegenprüfung der vier `_specs`-Crates (Black-Box) gegen die `.rules`"
status: open
---

# Gegenprüfung der vier `_specs`-Crates (Black-Box) gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#70](https://github.com/mbalzert1978/bank_auswertung/issues/70)

## What to build

Gegenprüfung der vier `_specs`-Crates — `crates/einlesen_specs`, `crates/obfuskierung_specs`,
`crates/fenstern_specs`, `crates/auswerten_specs` — gegen die nach `.rules/` ausgelagerten
Regeln. **Reines Audit** — je einschlägiger Regel ein Befund **pro Crate**; **kein Code wird
geändert**. Die vier Crates teilen dieselbe Regel-Menge (reine Black-Box-Testkultur), daher
gebündelt.

Einschlägige Regeln (für jede der vier `_specs`-Crates):

- `.rules/common/testing.md` — keine Format-Mechanik auf Ablehnung testen (deutsche
  Zahl/Datum ist `io`s Job); Presence-Enforcement als **Slice-Verhalten** testen; Verhalten
  ausschließlich durch die Fluent `*TestApi`; reine **Black-Box** (nur die öffentliche
  Adapter-Fläche: TestApi + DTOs + nackter Domänen-Fehler — keine Outbox-Typen, keine
  Interna). Slice-spezifisch beobachtbar: `fenstern` „leerer Store = kein Fehler",
  `auswerten` `SeedMarker`-Fehlschlag.
- `.rules/rust/testing-werkzeuge.md` — `proptest`/`cargo-fuzz` wo einschlägig.
- `.rules/crates/dependencies-und-kapselung.md` — `_specs` bindet **nicht**
  `application_kernel`; kein Import aus einem `_domain` oder aus Adapter-Interna.
- `.rules/common/sprache.md` — Testnamen/Doc-Kommentare Deutsch.
- `.rules/rust/gates.md`.

## Acceptance criteria

- [ ] Für **jede** der vier `_specs`-Crates jede gelistete Regel geprüft; je Regel + Crate ein Befund (konform / `file:line`).
- [ ] Belegt: keine Format-Mechanik-Ablehnung, keine Konstruktor-/Interna-/Outbox-Typ-Tests; Verhalten läuft durch die `*TestApi`.
- [ ] Belegt: kein `application_kernel`- und kein `_domain`-Import in den Specs.
- [ ] Redigierter Prüf-Report je Crate abgelegt; Abweichungen als Folge-Ticket-Vorschlag notiert — **kein Code geändert**.

## Blocked by

- None — kann sofort starten (unabhängiges Audit-Ticket).
