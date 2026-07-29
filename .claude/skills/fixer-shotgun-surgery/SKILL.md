---
name: fixer-shotgun-surgery
description: Apply the fix for the Shotgun Surgery code smell — consolidate one conceptual change scattered across many files into a single place. Use when a `verifier-shotgun-surgery` finding needs remediating, or directly asked to fix it — "fix this shotgun surgery", "diese verstreute Aenderung konsolidieren", "consolidate this change into one place".
arguments: Optional. What to fix — a `verifier-shotgun-surgery` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Shotgun Surgery Fixer

Applies refactoring.guru's fix for this smell: "a single change is made to
multiple classes simultaneously." Paired with `verifier-shotgun-surgery`, which
distinguishes a genuine structural gap from an inherently cross-cutting
change (a rename touching every call site); this skill applies the fix only
to the former.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **The same knowledge duplicated across files so a rule change requires
  editing each copy** → **Move Method** / **Move Field** to consolidate the
  knowledge into one place.
- **A thin pass-through class forcing the ripple through an extra hop** →
  **Inline Class** to remove the hop.

If the root cause is a type code switched on in several places, that's
`verifier-switch-statements`/`switch-statements-fixer`'s territory — apply that
fix instead of duplicating the consolidation here.
