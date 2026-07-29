---
name: fixer-long-parameter-list
description: Apply the fix for the Long Parameter List code smell — collapse a method/constructor's excess parameters into an object or a derivable lookup. Use when a `verifier-long-parameter-list` finding needs remediating, or directly asked to fix it — "fix this long parameter list", "diese Parameterliste kuerzen", "turn these params into an object".
arguments: Optional. What to fix — a `verifier-long-parameter-list` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Long Parameter List Fixer

Applies refactoring.guru's fix for this smell: more than three or four
parameters make a method hard to understand and easy to call wrong. Paired
with `verifier-long-parameter-list`, which flags a single long signature even
if it never repeats elsewhere; this skill applies the fix once a genuine
instance is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **A parameter derivable from another parameter already passed** →
  **Preserve Whole Object**: pass the richer object, let the callee pull
  what it needs.
- **A parameter that's really config/environment, not per-call data**
  (threaded through every layer to reach one distant branch) → **Replace
  Parameter with Method Call** if the callee can look it up itself, or hoist
  it out of the parameter list entirely.
- **A cohesive group of parameters that belongs together** → **Introduce
  Parameter Object**. If `verifier-data-clumps`/`data-clumps-fixer` already
  flagged the same group recurring across signatures, use that finding's
  extracted type here instead of creating a second one.
