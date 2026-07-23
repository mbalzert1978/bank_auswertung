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
Regeln. **Umsetzungs-Ticket** — der Code von `crates/auswerten_domain` wird gegen jede unten
gelistete einschlägige `.rules`-Regel geprüft und, **wo eine Regel verletzt ist, im Code
angepasst**, bis die Crate ihren `.rules` entspricht. Ist die Crate bereits konform, bleibt
sie unverändert — **keine künstliche Änderung, kein neues Ticket, kein Audit-Report**.
Redigiert bleibt Pflicht: keine rohen Identifikatoren/Klartexte in Commit/PR.

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

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] `Fingerabdruck` ist reine Domänenfunktion (nicht im Adapter/`io`); Kategorisierer nur bei Unbekanntem — sonst hergestellt.
- [ ] `Gegenkonto`-Auflösung auf Tokens, keine rohe IBAN; `SeedMarker`-Guard vorhanden — sonst ergänzt.
- [ ] `Gegenkonto` als Summentyp, `Betrag` vorzeichenlos; sonst umgestellt.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
