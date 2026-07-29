---
name: verifier-data-class
description: Review a diff/branch for anemic classes that hold only fields plus getters/setters and no behavior of their own, forcing every operation on their data to live in some other class instead. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Data Class code smell — "data class check", "ist das eine anaemische Klasse", "does this type have any behavior or just fields".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Data Class Check

Refactoring.guru's smell: "a data class refers to a class that contains only fields
and crude methods for accessing them... they lack independent functionality and cannot
operate on their own data." This is the anti-anemic-domain principle: a type that models
a domain concept is expected to expose behavior over its own state, not just storage.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **A domain type with only accessors and no methods that operate on its own state** —
  every decision about that data happens in a caller instead → **Move Method** the
  operation into the type itself; the caller should ask the object to act, not read its
  fields and decide externally.
- **Externally writable fields that let other code mutate state piecemeal**, bypassing
  any invariant → **Remove Setting Method** / **Encapsulate Field**, replacing free
  mutation with a behavior-bearing method that enforces the invariant.
- **A public collection field exposed for direct external mutation** →
  **Encapsulate Collection** so add/remove goes through the owning type.
- **Callers reaching in to compute something the class itself should compute** — this
  overlaps with `verifier-feature-envy`'s trigger from the caller's side; cross-reference
  rather than double-counting the same root cause.
- **A boundary type with only data and no behavior is *not* a finding** — a
  command, an event, a request/response DTO or a serialization shape is *supposed* to be
  inert data crossing a seam. This check applies to types that model domain concepts
  (value objects, entities, aggregate roots), not to the flat carriers around them.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Class | Fields but no behavior | Location | Fix |
| ----- | ------------------------ | -------- | --- |
| `Auswertungsfenster` | `von`/`bis` exposed as `pub` fields; the "day falls in window" logic lives in the caller | `datei:zeile` | Move Method: `Auswertungsfenster::enthaelt(tag)` instead of the caller reading `von`/`bis` directly |

Only list rows for domain types (value objects, entities, aggregates) with genuinely missing
behavior — boundary DTOs/Commands are out of scope. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
