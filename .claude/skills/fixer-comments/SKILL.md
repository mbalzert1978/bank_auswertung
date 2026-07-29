---
name: fixer-comments
description: Apply the fix for the Comments code smell — replace a WHAT-comment with a name that carries the meaning instead, leaving genuine WHY comments untouched. Use when a `verifier-comments` finding needs remediating, or directly asked to fix it — "fix this comment smell", "Kommentar durch Code ersetzen", "replace this comment with a better name", "remove this WHAT comment".
arguments: Optional. What to fix — a `verifier-comments` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Comments Fixer

Applies refactoring.guru's fix for this smell: comments are "usually created
with the best of intentions, when the author realizes that his or her code
isn't intuitive" — the fix is to make the code itself intuitive, not to keep
the comment. Paired with `verifier-comments`, which decides what counts as a
genuine WHAT-comment or crutch; this skill applies the fix once one is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **A comment restating the next line in prose** → delete it outright, or if
  the underlying code genuinely needs the name to carry the meaning,
  **Rename Method** / **Extract Variable** / **Extract Method** first, then
  delete the comment.
- **A block comment introducing a section of a long method** → **Extract
  Method** (cross-check with `long-method-fixer` first so the same method
  isn't extracted twice), then delete the comment.
- **A comment stating an invariant instead of enforcing it** → **Introduce
  Assertion** so the invariant is checked, not just documented and hoped for.
- **A comment referencing the current task/fix/issue number** → delete it;
  that content belongs in the commit message, where it stays tied to the
  change instead of going stale in the code.
- **A genuine WHY comment** (a non-obvious constraint, a workaround for a
  specific bug, a reason a seemingly-wrong choice is actually correct) →
  leave it exactly as is — never touch it, never "clean it up".

Only apply these to comments the paired check actually flagged — don't go
hunting for additional comments beyond what was located.
