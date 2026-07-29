---
name: fixer-dead-code
description: Apply the fix for the Dead Code smell — delete a variable, parameter, field, method, or class with zero remaining references. Use when a `verifier-dead-code` finding needs remediating, or directly asked to fix it — "fix this dead code", "toten Code entfernen", "delete this unused method/field".
arguments: Optional. What to fix — a `verifier-dead-code` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Dead Code Fixer

Applies refactoring.guru's fix for this smell: "a variable, parameter, field,
method or class is no longer used." Paired with `verifier-dead-code`, which
verifies zero remaining references across the whole repo (including tests)
before flagging; this skill applies the fix — deletion — once a genuine
instance is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **A method/field/class with zero remaining call sites/references** →
  delete it outright. Never comment it out, never rename it to `_unused` —
  the fix is removal, not quarantine.
- **A parameter accepted but never read in the method body** → **Remove
  Parameter**.
- **A class only ever referenced by other dead code** → **Inline Class** /
  **Collapse Hierarchy** to remove the whole chain at once, not one class at
  a time.
- **A leftover branch/case that can never execute after this diff** → delete
  it, even though it isn't a whole unused symbol.

Before deleting anything, re-verify zero references yourself (don't trust a
stale finding blindly) — in particular, confirm a non-public member isn't
still reachable from the test suite, which would make it live, not dead.
