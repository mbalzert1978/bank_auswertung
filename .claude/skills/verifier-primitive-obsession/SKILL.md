---
name: verifier-primitive-obsession
description: Review a diff/branch for raw primitives (string, int, bool) standing in for a concept that has its own rules — currency, ranges, phone numbers, type codes, string-keyed field names — instead of a small dedicated type. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Primitive Obsession code smell — "primitive obsession check", "ist das ein rohes primitive statt eines Value Objects", "sollte das ein eigener Typ sein statt string/int".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Primitive Obsession Check

Refactoring.guru's Primitive Obsession smell: using primitives instead of small objects
for simple tasks, relying on constants for coded values, or string-keyed field names
instead of proper structure. The principle underneath is *parse, don't validate*: a
concept that carries rules gets a type that enforces them once, at construction, so no
downstream caller has to re-check them.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **A primitive used for a concept with its own rules** — money amounts, ranges
  (start/end pairs), account identifiers, email addresses, date stamps — passed around
  as a bare string/integer/float with validation re-checked at each use site →
  **Replace Data Value with Object**: a dedicated type that validates once on
  construction.
- **A magic constant standing in for a type code** — an integer constant named
  `ROLE_ADMIN`, a raw string compared against literal values (`"admin"`, `"pending"`) →
  **Replace Type Code with Class**, **...with Subclasses**, or **...with
  State/Strategy** — or, where the language has one, a closed enumeration matched
  exhaustively.
- **String-keyed field names into a loosely-typed map/slice** (`row["Name"]`,
  `data["Status"]`) where a typed struct would let the compiler catch typos →
  **Replace Array with Object**.
- **The same handful of primitives always traveling together** through method
  signatures (e.g. `street, city, zip` everywhere) — that's `verifier-data-clumps`'s
  finding overlapping this one; note it there instead of duplicating. This check asks
  the narrower question of whether *one* value deserves a type of its own.
- **A raw primitive threaded through many layers just to reach one distant use** — a
  candidate for **Introduce Parameter Object** / **Preserve Whole Object** so callers
  pass the richer object instead of re-deriving a primitive from it.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Primitive | Concept it stands in for | Location | Fix |
| --------- | ------------------------- | -------- | --- |
| raw string for an account id | account identifier, re-validated at every call site | `datei:zeile` | dedicated `AccountId` type, validating on construction |

Only list rows where the primitive genuinely carries domain rules re-checked elsewhere —
a primitive that's a pure, ruleless scalar (a loop counter, a plain flag) isn't a finding.
End with:

```
Findings: <n>
```

`<n>` = count of concrete Primitive Obsession findings.
