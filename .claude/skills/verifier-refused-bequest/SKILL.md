---
name: verifier-refused-bequest
description: Review a diff/branch for a subclass that inherits from a parent but only uses some of its members — the rest sit unused, get overridden to throw, or get redefined to no-op, signaling the inheritance relationship itself is wrong. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Refused Bequest code smell — "refused bequest check", "nutzt die Subklasse die geerbten Member wirklich", "does this inheritance relationship actually make sense".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Refused Bequest Check

Refactoring.guru's smell: a subclass "uses only some of the methods and properties
inherited from its parents" — the hierarchy is off-kilter because the two types don't
truly share an is-a relationship.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **An override that raises "not supported"/"not implemented"** or is a deliberate
  no-op — the clearest signal the base class contract doesn't fit this subtype (a Liskov
  Substitution violation as much as a smell) → **Replace Inheritance
  with Delegation** (hold a reference to the base behavior instead of inheriting it) or
  **Extract Superclass** with only the genuinely shared members, moving the rest down.
- **Inherited public members the subclass never calls and never expects callers to
  call through it** — dead surface area inherited "for free" rather than because it's
  needed.
- **A subclass built purely to reuse one or two methods**, dragging in an entire base
  contract it doesn't otherwise want → same fix: prefer delegation/composition over
  inheriting the whole shape for a fraction of it.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Subclass | Parent | Refused members | Location | Fix |
| -------- | ------ | ---------------- | -------- | --- |
| `ReadOnlyStore` | `Store` | Overrides `Remove`/`Add` to throw | `datei:zeile` | Extract narrower base with only the read members; delegate instead of inheriting the mutating ones |

Only list rows with genuine unused/refused/no-op members — a subclass using most of its
parent with one narrow, justified override isn't a finding. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
