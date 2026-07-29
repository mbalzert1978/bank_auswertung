---
name: fixer-alternative-classes-with-different-interfaces
description: Apply the fix for the Alternative Classes with Different Interfaces code smell — align two classes that do the same job under different method names/signatures so callers can treat them interchangeably. Use when a `verifier-alternative-classes-with-different-interfaces` finding needs remediating, or directly asked to fix it — "fix alternative classes with different interfaces", "vereinheitliche diese zwei Klassen", "align these two classes' interfaces", "make these interchangeable".
arguments: Optional. What to fix — a `verifier-alternative-classes-with-different-interfaces` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Alternative Classes with Different Interfaces Fixer

Applies refactoring.guru's fix for this smell: "two classes perform identical
functions but have different method names." Paired with
`verifier-alternative-classes-with-different-interfaces`, which decides what
counts as a genuine instance; this skill applies the fix once one is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **Same job, different method names** → **Rename Method**: align the names
  across both classes.
- **Same job, different parameter shape** → **Add Parameter** /
  **Parameterize Method**: bring the signatures in line.
- **Logic that belongs together split across the two classes for historical
  reasons** → **Move Method**: consolidate before renaming.
- **A caller still switches on concrete type after alignment** →
  **Extract Superclass** (or whatever the language offers for a shared
  convention) so callers depend on the common shape, not the pair of
  lookalikes.

Only apply these to genuinely interchangeable classes (per the paired
check's own scope rules) — don't merge two classes that just sound similar
but serve materially different responsibilities.
