---
id: "0063"
title: "Gegenprüfung `obfuskierung_domain` gegen die `.rules`"
status: open
---

# Gegenprüfung `obfuskierung_domain` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#62](https://github.com/mbalzert1978/bank_auswertung/issues/62)

## What to build

Gegenprüfung des Codes von `crates/obfuskierung_domain` gegen die nach `.rules/`
ausgelagerten Regeln. **Reines Audit** — je einschlägiger Regel ein Befund; **kein Code wird
geändert**.

Einschlägige Regeln (der Privacy-Kern — hier liegen die härtesten Invarianten):

- `.rules/common/privacy-invarianten.md` — Freitext nur über `ObfuskierterText` (best-effort
  scrubben, ehrlich als heuristisch benannt, keine Anonymisierung behaupten); Fail-safe „im
  Zweifel `Person`"; der **Seed bleibt lokal** (nie in einem Egress-DTO); ab hier fließen nur
  Tokens, keine strukturellen Identifikatoren; Pseudonym-Stabilität seed-basiert ohne
  Klartext↔Token-Store.
- `.rules/common/domaenenmodell.md` — Value Objects (`Pseudonym`), Summentypen, deklarativ.
- `.rules/rust/idiome.md` — `_domain` dependency-frei (kein `serde`); Newtypes validieren.
- `.rules/common/architektur.md` — `Obfuscator`-Port; durchweg terminal; Aggregat referenziert
  die Quelle nur per ID.
- `.rules/crates/dependencies-und-kapselung.md`, `.rules/rust/performance.md`,
  `.rules/common/sprache.md`, `.rules/rust/gates.md`.

## Acceptance criteria

- [ ] Jede gelistete Regel gegen den Crate-Code geprüft; je Regel ein Befund (konform / `file:line`).
- [ ] Belegt: Seed bleibt im Prozess/lokal, erreicht keine ausgehende Struktur; kein Klartext↔Token-Store.
- [ ] Belegt: Freitext läuft ausschließlich durch `ObfuskierterText`; Fail-safe-`Person`-Pfad vorhanden.
- [ ] Dependency-Freiheit der Domäne verifiziert.
- [ ] Redigierter Prüf-Report abgelegt; Abweichungen als Folge-Ticket-Vorschlag notiert — **kein Code geändert**.

## Blocked by

- None — kann sofort starten (unabhängiges Audit-Ticket).
