---
id: "0067"
title: "Gegenprüfung `auswerten_domain` gegen die `.rules`"
status: open
---

# Gegenprüfung `auswerten_domain` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#66](https://github.com/mbalzert1978/bank_auswertung/issues/66)

## What to build

Gegenprüfung des Codes von `crates/auswerten_domain` gegen die nach `.rules/` ausgelagerten
Regeln. **Reines Audit** — je einschlägiger Regel ein Befund; **kein Code wird geändert**.

Einschlägige Regeln (der Analyse-Kern):

- `.rules/common/architektur.md` — **Die Domäne ist die Analyse, das LLM nur ein Orakel**
  (Lackmustest: ohne LLM bleibt das Programm wertvoll; `Kategorisierer` nur für **unbekannte**
  Fingerabdrücke); Ports; durchweg terminal.
- `.rules/common/privacy-invarianten.md` — **Fingerabdruck ist eine reine Domänenfunktion**
  (Firma via `GläubigerId`, Zahlungsdienstleister zusätzlich per Merchant-Token, Privatperson
  per Pseudonym); Fail-safe `Person`; `Gegenkonto::Eigen|Fremd` **auf Tokens** (keine rohe
  IBAN erreicht den Slice) mit `SeedMarker`-Guard; jeder Port sein schmales DTO (Antwort auf
  den **Fingerabdruck, nie den Text**).
- `.rules/common/domaenenmodell.md` — `Gegenkonto` als Summentyp (`Eigen(Rolle)|Fremd`);
  `Betrag` vorzeichenlos; deklarativ (`fold`/`filter`/`sum` für Cashflow/Occurrences).
- `.rules/rust/idiome.md` — `_domain` dependency-frei; Newtypes.
- `.rules/rust/performance.md` — Zero-Cost-Iteratoren in beiden Reaches (per-window &
  window-spanning).
- `.rules/rust/testing-werkzeuge.md`, `.rules/crates/dependencies-und-kapselung.md`,
  `.rules/common/sprache.md`, `.rules/rust/gates.md`.

## Acceptance criteria

- [ ] Jede gelistete Regel gegen den Crate-Code geprüft; je Regel ein Befund (konform / `file:line`).
- [ ] Belegt: `Fingerabdruck` ist reine Domänenfunktion (nicht im Adapter/`io`); Kategorisierer nur bei Unbekanntem.
- [ ] Belegt: `Gegenkonto`-Auflösung auf Tokens, keine rohe IBAN; `SeedMarker`-Guard vorhanden.
- [ ] Belegt: `Gegenkonto` als Summentyp, `Betrag` vorzeichenlos.
- [ ] Redigierter Prüf-Report abgelegt; Abweichungen als Folge-Ticket-Vorschlag notiert — **kein Code geändert**.

## Blocked by

- None — kann sofort starten (unabhängiges Audit-Ticket).
