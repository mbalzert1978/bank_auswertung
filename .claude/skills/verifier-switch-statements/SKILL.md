---
name: verifier-switch-statements
description: Review a diff/branch for a complex switch/if-chain on a type code that recurs (the same discriminant switched on in more than one place), which polymorphism or a closed sum type would replace. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Switch Statements code smell — "switch statements check", "wird hier auf einen Typ-Code verzweigt statt Polymorphie zu nutzen", "repeated switch on the same discriminant".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Switch Statements Check

Refactoring.guru's smell: "a complex switch operator or sequence of if statements."
The trigger is specifically **the same discriminant switched/if-chained in more than one
place** — the classic sign that a case is missing a home of its own and the branching
should move into the type itself. A single switch, in one place, is not this finding.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **The same enum/type-code switched on in two or more separate locations** — every new
  case then requires editing all of them (this is also `verifier-shotgun-surgery`'s
  trigger from the change-impact angle) → **Replace Type Code with Subclasses** or
  **Replace Type Code with State/Strategy**, or a closed sum type carrying the
  branch-specific behavior on each case.
- **One exhaustive match over a closed set, in exactly one place** — not a finding.
  Converting a closed type into its outward representation at a single boundary is the
  intended, centralized shape; the smell is the *repetition* of the discriminant, not
  the existence of a match.
- **A parameter used only to select which of several near-identical methods to run** →
  **Replace Parameter with Explicit Methods**.
- **A branch whose only job is "handle the absent/none case"** → **Introduce Null
  Object** instead of an explicit null-check branch repeated at each call site.
- **Method/logic misplaced relative to the type it switches on** → **Move Method**,
  **Extract Method** first, to gather the scattered logic before converting it to
  polymorphism.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Discriminant | Switched/if-chained in | Location(s) | Fix |
| ------------ | ------------------------ | ------------ | --- |
| `ReservationKind` | 3 places (converter, handler, presenter) | `datei:zeile`, `datei:zeile`, `datei:zeile` | Move the per-case logic onto the cases themselves; keep one exhaustive match, at the outward conversion boundary |

Only list rows for a discriminant switched on in **more than one location**, or a
genuinely open-ended (non-exhaustive, likely-to-grow) switch. A single, exhaustive,
centrally-located switch is the accepted pattern here, not a finding. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
