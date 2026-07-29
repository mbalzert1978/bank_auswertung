---
name: fixer-divergent-change
description: Apply the fix for the Divergent Change code smell — split a class that changes for many unrelated reasons into one class per reason. Use when a `verifier-divergent-change` finding needs remediating, or directly asked to fix it — "fix this divergent change", "diese Klasse nach Verantwortlichkeiten aufteilen", "split this class by reason to change".
arguments: Optional. What to fix — a `verifier-divergent-change` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Divergent Change Fixer

Applies refactoring.guru's fix for this smell: "Divergent Change is when many
changes are made to a single class" — one new feature/case forcing edits to
several of its unrelated methods. Paired with `verifier-divergent-change`,
which decides what counts as a genuine unrelated-reasons pattern; this skill
applies the fix once one is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **One diff touching several unrelated methods of the same class for what
  is really one new feature/case** → **Extract Class** per concern, so
  future additions of the same kind touch only one place.
- **A class whose method list reads as several unrelated verbs** ("parses
  X", "renders Y", "persists Z") → **Extract Superclass** / **Extract
  Subclass** to separate the variant-specific parts from the shared shape.

If another check already flagged the same class for the same root cause
(`verifier-large-class` from the class-shape angle, `verifier-shotgun-surgery`
from the mirror-image angle), apply one fix, not two — don't duplicate the
split.
