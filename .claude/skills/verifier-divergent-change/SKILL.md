---
name: verifier-divergent-change
description: Review a diff/branch for a single class that has to change for many unrelated reasons — adding one new case forces edits to several of its unrelated methods. The mirror image of Shotgun Surgery: many reasons converging on one class instead of one reason scattering across many classes. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Divergent Change code smell — "divergent change check", "aendert sich diese Klasse aus zu vielen Gruenden", "does this class have too many reasons to change".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Divergent Change Check

Refactoring.guru's smell: "Divergent Change is when many changes are made to a single
class" — e.g. adding a new product type forces edits to that class's finding,
displaying, *and* ordering methods, because all three unrelated concerns live in one
type. This is the Single-Responsibility question judged from the **diff's actual edit
pattern** — does *this* change touch several unrelated methods of one class for
unrelated reasons — rather than from static structure alone. Where the same class is
also oversized, that's `verifier-large-class`'s angle; cross-reference rather than
double-report.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **One diff/commit touching several unrelated methods of the same class** for what is
  really one new feature/case (e.g. adding a case requires edits to `Find`, `Display`,
  and `Order` methods all on one class) → **Extract Class** per concern so future
  additions of the same kind touch only one place.
- **A class whose method list reads as several unrelated verbs** ("parses X", "renders
  Y", "persists Z") rather than one cohesive responsibility → **Extract Superclass** /
  **Extract Subclass** to separate the variant-specific parts from the shared shape.
- **A recurring pattern across git history** (if visible in the diff/PR context) where
  the same class shows up in unrelated feature PRs each time — a strong signal even
  before counting methods in a single diff.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Class | Unrelated reasons found | Location | Fix |
| ----- | ------------------------ | -------- | --- |
| `ProductCatalog` | Edited for lookup-format change AND display-format change AND ordering-rule change in one PR | `datei:zeile` | Extract Class per concern: `ProductFinder`, `ProductPresenter`, `ProductOrdering` |

Only list rows where the diff itself shows unrelated concerns being edited together —
a class touched once for one coherent reason isn't a finding. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
