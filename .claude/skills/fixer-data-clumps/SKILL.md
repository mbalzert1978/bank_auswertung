---
name: fixer-data-clumps
description: Apply the fix for the Data Clumps code smell — extract a recurring group of variables/parameters into its own type instead of passing them around separately. Use when a `verifier-data-clumps` finding needs remediating, or directly asked to fix it — "fix this data clump", "diese Parametergruppe zusammenfassen", "extract these fields into a value object".
arguments: Optional. What to fix — a `verifier-data-clumps` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Data Clumps Fixer

Applies refactoring.guru's fix for this smell: "different parts of the code
contain identical groups of variables... these clumps should be turned into
their own classes." Paired with `verifier-data-clumps`, which requires the
group to recur across at least two locations before flagging it; this skill
applies the fix once a genuine clump is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **The same 2+ variables appearing together as parameters in more than one
  method** → **Extract Class** (a small type validating its invariant once on
  construction), then **Introduce Parameter Object** at each call site.
- **The same group appearing as sibling fields on more than one class** →
  **Extract Class** so both classes hold a reference to the one new type
  instead of duplicating the fields.

Only fold in the call sites the paired check actually located — if the fix
surfaces further occurrences of the same clump elsewhere, note them in the
report rather than silently expanding scope.
