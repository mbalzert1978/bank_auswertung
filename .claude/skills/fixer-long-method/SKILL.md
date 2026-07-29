---
name: fixer-long-method
description: Apply the fix for the Long Method code smell — extract cohesive pieces out of a method that mixes several responsibilities or abstraction levels in one body. Use when a `verifier-long-method` finding needs remediating, or directly asked to fix it — "fix this long method", "diese Methode aufteilen", "split up this method", "extract methods here".
arguments: Optional. What to fix — a `verifier-long-method` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Long Method Fixer

Applies refactoring.guru's fix for this smell: "any method longer than ten
lines should make you start asking questions." Paired with
`verifier-long-method`, which judges length as only a trigger — the real
defect is mixed abstraction levels or responsibilities; this skill applies
the fix once a genuine instance is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **Mixed levels of abstraction in one body** (high-level orchestration next
  to low-level detail) → **Extract Method** per level, so the outer method
  reads like a summary.
- **A comment introducing a section** → **Extract Method** named for that
  section, then delete the comment (cross-check `comments-fixer` so it isn't
  applied twice).
- **A local variable recomputed or threaded through many lines** → **Replace
  Temp with Query**, or extract it into its own method.
- **Long, uninterrupted conditional/looping logic doing distinct jobs** →
  **Decompose Conditional** — first apply **Introduce Parameter Object** /
  **Preserve Whole Object** if a large related group of parameters/locals is
  what's blocking a clean extraction.
- **A method too tangled to extract cleanly** (shared mutable locals
  everywhere) → **Replace Method with Method Object** so the locals become
  fields of a small object built just for this one call.

Extract along the boundaries the paired check actually named — don't
reorganize a method beyond what the located finding covers.
