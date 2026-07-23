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
Regeln. **Umsetzungs-Ticket** — der Code der vier `_specs`-Crates wird gegen jede unten
gelistete einschlägige `.rules`-Regel geprüft und, **wo eine Regel verletzt ist, im Code
angepasst**, bis jede Crate ihren `.rules` entspricht. Ist eine Crate bereits konform, bleibt
sie unverändert — **keine künstliche Änderung, kein neues Ticket, kein Audit-Report**.
Redigiert bleibt Pflicht: keine rohen Identifikatoren/Klartexte in Commit/PR. Die vier Crates
teilen dieselbe Regel-Menge (reine Black-Box-Testkultur), daher gebündelt.

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

- [ ] Für **jede** der vier `_specs`-Crates ist jede gelistete Regel erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] Keine Format-Mechanik-Ablehnung, keine Konstruktor-/Interna-/Outbox-Typ-Tests; Verhalten läuft durch die `*TestApi` — sonst umgestellt.
- [ ] Kein `application_kernel`- und kein `_domain`-Import in den Specs; sonst entfernt.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist eine Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
