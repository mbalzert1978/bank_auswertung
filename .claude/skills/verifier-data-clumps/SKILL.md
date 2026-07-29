---
name: verifier-data-clumps
description: Review a diff/branch for the same small group of variables/parameters (e.g. host/port/credentials, street/city/zip) recurring together across multiple signatures or fields, which should be its own class instead of a repeated bundle. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Data Clumps code smell — "data clumps check", "wandern diese Felder immer zusammen", "should this trio of params be its own type".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Data Clumps Check

Refactoring.guru's Data Clumps smell: "different parts of the code contain identical
groups of variables... these clumps should be turned into their own classes." The
signal is *recurrence across at least two locations* — a single signature with a
related group is `verifier-long-parameter-list`'s territory, not this one.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **The same 2+ variables appearing together as parameters in more than one method** —
  e.g. every persistence-facing call taking `(amount, currency, direction)` separately
  instead of one money type → **Extract Class** (a small type that validates its
  invariant once on construction), then **Introduce Parameter Object** at each call site.
- **The same group appearing as sibling fields on more than one type** — two unrelated
  types each holding their own `from`/`to`/`length` fields → **Extract Class** so both
  hold one range value instead.
- **A group that always changes together** — if one of the three ever changes, the other
  two are edited in the same commit/PR — is a stronger signal than mere textual
  co-occurrence; call this out explicitly when visible in the diff.
- **Don't flag isolated occurrence** — a group of variables appearing together exactly
  once is not yet a clump; the finding requires at least two call sites/classes.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Variable group | Where it recurs | Location(s) | Fix |
| -------------- | ---------------- | ------------ | --- |
| `amount, currency, direction` | 3 signatures across 2 files | `datei:zeile`, `datei:zeile` | Extract a `Money` type; Introduce Parameter Object at each call site |

Only list rows with genuine repetition (2+ locations). End with:

```
Findings: <n>
```

`<n>` = count of concrete Data Clumps findings.
