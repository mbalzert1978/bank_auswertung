---
name: verifier-feature-envy
description: Review a diff/branch for a method that accesses another object's data/fields more than its own, meaning the logic is living in the wrong place. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Feature Envy code smell — "feature envy check", "greift diese Methode mehr auf fremde Daten zu als auf eigene", "should this logic move to the object it's really operating on".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Feature Envy Check

Refactoring.guru's smell: "a method accesses the data of another object more than its
own data." That is the anti-anemic-domain principle violated from the caller's side: a
caller pattern-matching on or reading another object's state to make a decision that
object should make itself.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **A method that calls more getters/reads more fields on a parameter/collaborator
  than on `this`** — the clearest quantitative signal → **Move Method** to the class
  whose data it's actually using.
- **A method that pattern-matches/switches on another object's internal state from
  outside** (`if order.status() == Status.Cancelled` scattered at call sites) instead of
  asking that object — this is the same root cause `verifier-data-class` flags from the
  callee's side; cross-reference rather than double-counting.
- **A calculation over a collaborator's fields that's repeated at more than one call
  site** → **Extract Method** first, then **Move Method** onto the collaborator so the
  calculation lives with the data it needs.
- **Partial envy is still worth flagging** — a method split evenly between its own data
  and one collaborator's is a weaker finding than one that's almost entirely about the
  collaborator; use judgment on degree, not a strict majority rule.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Method | Envies | Location | Fix |
| ------ | ------ | -------- | --- |
| `WindowEvaluationHandler::cashflow` | Reads 4 fields of `Auswertungsfenster`, none of its own | `datei:zeile` | Move Method nach `Auswertungsfenster::cashflow` |

Only list rows with a genuine, substantial lean toward another object's data — a
method that legitimately coordinates two objects' data isn't a finding. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
