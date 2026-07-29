---
name: fixer-large-class
description: Apply the fix for the Large Class code smell — split a class whose fields/methods cluster into more than one natural group. Use when a `verifier-large-class` finding needs remediating, or directly asked to fix it — "fix this large class", "diese Klasse aufteilen", "split this god class".
arguments: Optional. What to fix — a `verifier-large-class` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Large Class Fixer

Applies refactoring.guru's fix for this smell: "a class contains many
fields/methods/lines of code." Paired with `verifier-large-class`, which
requires a genuinely separable field/method cluster before flagging (a big
but still-cohesive class is not a finding there); this skill applies the fix
once one is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **Field clusters never used together by the same methods** → **Extract
  Class** per cluster — that's two classes pretending to be one.
- **One job with two variant strategies bolted on as flags/branches** →
  **Extract Subclass** for the variant behavior instead of an `if (mode ==
  X)` sprinkled through the class.
- **More public surface than any one collaborator needs** → **Extract
  Interface** so each caller depends only on the slice it actually needs.
- **Parallel data arrays/observed-data duplicated per instance** where a
  proper object per data point would do → **Duplicate Observed Data**.

Only split along the boundary the paired check actually identified — don't
extract a cluster the finding didn't name.
