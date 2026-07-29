---
name: fixer-parallel-inheritance-hierarchies
description: Apply the fix for the Parallel Inheritance Hierarchies code smell — merge two hierarchies that must be extended in lockstep into one. Use when a `verifier-parallel-inheritance-hierarchies` finding needs remediating, or directly asked to fix it — "fix this parallel inheritance", "diese zwei Hierarchien zusammenfuehren", "merge these mirrored subclass pairs".
arguments: Optional. What to fix — a `verifier-parallel-inheritance-hierarchies` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Parallel Inheritance Hierarchies Fixer

Applies refactoring.guru's fix for this smell: "whenever you create a
subclass for a class, you find yourself needing to create a subclass for
another class." Paired with `verifier-parallel-inheritance-hierarchies`, which
excludes deliberate per-feature conventions (a request with its handler and
its response, a model with its migration) from counting as findings; this
skill applies the fix only to a genuine one-for-one mirrored *inheritance*
pair with no compression opportunity.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **A subclass in the second hierarchy that does nothing but mirror/delegate
  to its counterpart in the first** → **Move Method** / **Move Field** to
  merge the mirrored behavior into one hierarchy, letting the second
  reference or be instantiated by the first instead of shadowing it
  one-for-one.

Leave a deliberate per-feature parallel structure alone — a repeated but
intentional composition convention is not this smell.
