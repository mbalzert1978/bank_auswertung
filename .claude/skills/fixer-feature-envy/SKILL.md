---
name: fixer-feature-envy
description: Apply the fix for the Feature Envy code smell — move a method to the class whose data it actually operates on. Use when a `verifier-feature-envy` finding needs remediating, or directly asked to fix it — "fix this feature envy", "diese Methode zum richtigen Objekt verschieben", "move this method to where its data lives".
arguments: Optional. What to fix — a `verifier-feature-envy` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Feature Envy Fixer

Applies refactoring.guru's fix for this smell: "a method accesses the data of
another object more than its own data." That is the anti-anemic-domain
principle violated from the caller's side: the caller decides something the
object should decide itself. Paired with `verifier-feature-envy`, which
decides how much lean toward
another object's data counts as a genuine instance; this skill applies the
fix once one is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **A method that calls more getters/reads more fields on a
  parameter/collaborator than on `this`** → **Move Method** to the class
  whose data it's actually using.
- **A calculation over a collaborator's fields repeated at more than one
  call site** → **Extract Method** first, then **Move Method** it onto the
  collaborator so the calculation lives with the data it needs.

If `verifier-data-class`/`data-class-fixer` already flagged the same root cause
from the callee's side (the class lacking the behavior), apply one fix, not
two — moving the method in also closes the data-class gap.
