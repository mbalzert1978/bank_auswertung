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
ausgelagerten Regeln. **Umsetzungs-Ticket** — der Code von `crates/obfuskierung_domain` wird
gegen jede unten gelistete einschlägige `.rules`-Regel geprüft und, **wo eine Regel verletzt
ist, im Code angepasst**, bis die Crate ihren `.rules` entspricht. Ist die Crate bereits
konform, bleibt sie unverändert — **keine künstliche Änderung, kein neues Ticket, kein
Audit-Report**. Redigiert bleibt Pflicht: keine rohen Identifikatoren/Klartexte in Commit/PR.

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

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] Seed bleibt im Prozess/lokal, erreicht keine ausgehende Struktur; kein Klartext↔Token-Store — sonst hergestellt.
- [ ] Freitext läuft ausschließlich durch `ObfuskierterText`; Fail-safe-`Person`-Pfad vorhanden — sonst ergänzt.
- [ ] Dependency-Freiheit der Domäne; fehlte sie, hergestellt.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
