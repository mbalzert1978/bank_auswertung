---
name: verifier-duplicate-code
description: Review a diff/branch for two or more near-identical code fragments — the same logic copy-pasted (possibly with minor variable renames) instead of shared. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Duplicate Code smell — "duplicate code check", "gibt es hier fast identischen Code an zwei Stellen", "should this be shared instead of copy-pasted".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Duplicate Code Check

Refactoring.guru's smell: "two code fragments look almost identical." This is a
structural finding, not a lint: no formatter or style tool detects it, because the
duplicated fragments are each individually well-formed. This check reads the diff itself
for copy-pasted logic.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **The same logic repeated in two sibling classes** → **Pull Up Field** / **Pull Up
  Constructor Body** / **Form Template Method** if they share a base, or **Extract
  Superclass** to create one.
- **The same logic repeated with no shared base** → **Extract Class** or **Extract
  Method** into a shared helper both call.
- **Two algorithms that do the same job slightly differently** (one path handles an
  edge case the other doesn't) → **Substitute Algorithm** — replace both with one
  correct version rather than maintaining two.
- **Duplicated conditional structure** — the same `if`/`else` shape repeated with minor
  variations, or the same condition checked redundantly nearby → **Consolidate
  Conditional Expression** / **Consolidate Duplicate Conditional Fragments**.
- **Copy-paste with renamed variables/fields** is still duplication — don't let a
  surface-level rename hide the finding; look for structurally identical bodies.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Fragment A | Fragment B | Similarity | Fix |
| ---------- | ---------- | ----------- | --- |
| `WindowEvaluationHandler::fingerabdruecke` | `HistoryEvaluationHandler::fingerabdruecke` | Same fingerprint derivation, copy-pasted | `datei:zeile` / `datei:zeile` — Extract Method in eine gemeinsame Domänenfunktion |

Only list rows for genuinely near-identical fragments — structurally similar code that
solves a materially different problem isn't a finding. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
