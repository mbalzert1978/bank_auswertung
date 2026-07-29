---
name: fixer-duplicate-code
description: Apply the fix for the Duplicate Code smell — replace near-identical copy-pasted fragments with one shared implementation. Use when a `verifier-duplicate-code` finding needs remediating, or directly asked to fix it — "fix this duplicate code", "diesen doppelten Code zusammenfuehren", "extract this shared logic".
arguments: Optional. What to fix — a `verifier-duplicate-code` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Duplicate Code Fixer

Applies refactoring.guru's fix for this smell: "two code fragments look
almost identical." Paired with `verifier-duplicate-code`, which decides what
counts as genuine near-identical duplication (not merely structurally
similar code solving a different problem); this skill applies the fix once
one is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **The same logic repeated in two sibling classes with a shared base** →
  **Pull Up Field** / **Pull Up Constructor Body** / **Form Template
  Method**; if there's no base yet, **Extract Superclass** first.
- **The same logic repeated with no shared base** → **Extract Class** or
  **Extract Method** into a shared helper both call.
- **Two algorithms doing the same job slightly differently** (one handles an
  edge case the other doesn't) → **Substitute Algorithm** — replace both
  with one correct version rather than maintaining two.
- **Duplicated conditional structure** → **Consolidate Conditional
  Expression** / **Consolidate Duplicate Conditional Fragments**.

A copy-paste with renamed variables/fields is still duplication — don't let
a surface-level rename stop you from consolidating it.
