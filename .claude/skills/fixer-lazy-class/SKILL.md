---
name: fixer-lazy-class
description: Apply the fix for the Lazy Class code smell — inline or collapse a class that no longer does enough to earn its own existence. Use when a `verifier-lazy-class` finding needs remediating, or directly asked to fix it — "fix this lazy class", "diese Klasse einsparen", "inline this class".
arguments: Optional. What to fix — a `verifier-lazy-class` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Lazy Class Fixer

Applies refactoring.guru's fix for this smell: "understanding and maintaining
classes always costs time and money," so a class reduced to almost nothing
should go away. Paired with `verifier-lazy-class`, which excludes
intentionally-thin types (a value object, a marker type, a narrow-but-complete
case of a closed set) from counting as a finding; this skill applies the fix
once a genuine instance is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **A class with one field and one trivial method**, inlineable at its call
  sites → **Inline Class**.
- **A class left behind after a refactor stripped out most of its
  responsibilities** → **Inline Class** / **Collapse Hierarchy** into
  whatever now holds the remaining responsibility.
- **A once-planned extension point that never grew a second implementation**
  → **Collapse Hierarchy**.

If `verifier-middle-man`/`middle-man-fixer` already flagged the same class as a
pure delegator, apply that fix instead — a lazy class that does nothing but
forward is the middle-man variant specifically, not a separate fix.
