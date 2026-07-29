---
name: fixer-speculative-generality
description: Apply the fix for the Speculative Generality code smell — remove an abstraction built "just in case" that no current caller needs. Use when a `verifier-speculative-generality` finding needs remediating, or directly asked to fix it — "fix this speculative generality", "diese ungenutzte Abstraktion entfernen", "YAGNI cleanup", "collapse this one-case wrapper".
arguments: Optional. What to fix — a `verifier-speculative-generality` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Speculative Generality Fixer

Applies refactoring.guru's fix for this smell: code created "just in case"
to support anticipated future features that never materialize. The bar: an
abstraction earns its place by serving a need that exists now, not one that
might. Paired
with `verifier-speculative-generality`, which excludes genuinely-used
extensibility (2+ real implementations or callers) from counting as a
finding; this skill applies the fix only to abstractions with zero current
second use.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **An unused class, method, field, or parameter introduced in this diff
  with no current caller** → **Remove Parameter** / **Inline Method** /
  **Inline Class**.
- **An interface/abstract base with exactly one implementation and no second
  one planned** → **Collapse Hierarchy**.
- **A result/outcome wrapper with a single case, or an error type for a
  failure mode the operation cannot have** → collapse it, return the
  underlying value directly.
- **A configuration knob, extension point, or strategy parameter with only
  one value ever passed** → remove the parameter, hard-code the one real
  behavior until a second case actually exists.

Never remove genuine, currently-used extensibility — an interface with two
or more real implementations, or a parameter two or more callers already
pass differently, is not this smell even if it looks generic.
