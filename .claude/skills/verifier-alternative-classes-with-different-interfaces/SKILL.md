---
name: verifier-alternative-classes-with-different-interfaces
description: Review a diff/branch for two classes that perform the same job but expose it under different method names/signatures, so callers can't treat them interchangeably even though conceptually they're the same abstraction. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for this code smell — "alternative classes with different interfaces check", "tun diese zwei Klassen dasselbe unter anderem Namen", "should these share an interface".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Alternative Classes with Different Interfaces Check

Refactoring.guru's smell: "two classes perform identical functions but have different
method names." The functional overlap is hidden by naming, so callers end up
special-casing which concrete type they hold instead of programming to a shared shape.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **Two classes/gateways/repositories doing the same conceptual job with different
  method names** — e.g. one type exposes `Find(id)` and a sibling exposes
  `GetById(id)` for the same kind of lookup → **Rename Method** to align them.
- **Same job, different parameter shape** — one takes a raw id, the other takes a
  wrapping object for the same concept → **Add Parameter** / **Parameterize Method**
  to bring the signatures in line.
- **Logic that belongs together split across the two classes for historical reasons**
  → **Move Method** to consolidate before renaming.
- **Once aligned, a caller still switches on concrete type to pick one or the other**
  → **Extract Superclass**, or whatever the language offers for a shared contract
  (interface, protocol, trait), so callers depend on the common shape rather than on the
  pair of lookalikes.
- **Don't flag genuinely different responsibilities that merely sound similar** — two
  classes are only a finding here if they are interchangeable in practice, not just
  similarly named.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Classes | Same job, different interface | Location | Fix |
| ------- | ------------------------------ | -------- | --- |
| `LeaseRepository` / `ReservationRepository` | Both "find by MAC", named `FindByMac` vs `LookupMac` | `datei:zeile`, `datei:zeile` | Rename Method to align; extract shared interface if a caller needs both |

Only list rows for genuine functional duplication under different names. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
