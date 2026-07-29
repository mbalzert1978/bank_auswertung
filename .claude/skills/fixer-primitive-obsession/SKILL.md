---
name: fixer-primitive-obsession
description: Apply the fix for the Primitive Obsession code smell — replace a raw primitive standing in for a domain concept with a small dedicated type. Use when a `verifier-primitive-obsession` finding needs remediating, or directly asked to fix it — "fix this primitive obsession", "dieses Primitive in ein Value Object umwandeln", "wrap this primitive in a type".
arguments: Optional. What to fix — a `verifier-primitive-obsession` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Primitive Obsession Fixer

Applies refactoring.guru's fix for this smell: using primitives instead of
small objects for simple tasks, relying on constants for coded values, or
string-keyed field names instead of proper structure. The principle underneath
is *parse, don't validate*: the new type enforces the concept's rules once, on
construction, so no caller re-checks them. Paired with `verifier-primitive-obsession`,
which requires the primitive to genuinely carry re-checked domain rules
(not a pure, ruleless scalar); this skill applies the fix once a genuine
instance is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **A primitive used for a concept with its own rules** (money, ranges,
  identifiers, email addresses, date stamps) validated at each use site →
  **Replace Data Value with Object**: a dedicated type validating once on
  construction.
- **A magic constant standing in for a type code** (a raw string/int
  compared against literal values) → **Replace Type Code with Class**,
  **...with Subclasses**, or **...with State/Strategy** — or, where the
  language offers one, a closed enumeration matched exhaustively.
- **String-keyed field names into a loosely-typed map/collection** →
  **Replace Array with Object** so a typo becomes a compile/lookup error
  rather than a silent miss.
- **A raw primitive threaded through many layers just to reach one distant
  use** → **Introduce Parameter Object** / **Preserve Whole Object** so
  callers pass the richer object instead of re-deriving the primitive.

If the same primitives always travel together across signatures, that's
`verifier-data-clumps`/`data-clumps-fixer`'s territory — use its extracted
type here instead of introducing a second one.
