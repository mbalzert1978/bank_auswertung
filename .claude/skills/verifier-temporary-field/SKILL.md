---
name: verifier-temporary-field
description: Review a diff/branch for a field that's only meaningfully set during one specific algorithm/call and sits empty/unused the rest of the time, which confuses readers about what state the object is actually supposed to hold. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Temporary Field code smell — "temporary field check", "ist dieses Feld nur waehrend eines Algorithmus befuellt", "field that's empty outside one method".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Temporary Field Check

Refactoring.guru's smell: "temporary fields get their values... only under certain
circumstances. Outside of these circumstances, they're empty." Usually created so a
large algorithm can avoid passing many parameters between its own private helper
methods, at the cost of making the object's real invariants unclear.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **A field only assigned inside one method (or one small cluster of private helpers)
  and read nowhere else** — outside that call, the field is meaningless/stale/null →
  **Extract Class**: pull that method plus its temporary fields into a small object
  created just for the one algorithm's duration.
- **A field used as a stand-in for what should be several method parameters** — set
  right before a chain of private calls, read by each of them in turn → same fix, or
  **Replace Method with Method Object** if the whole call chain is one logical
  operation.
- **Conditional logic scattered around checking "is this field set right now"** before
  using it — a symptom that the field's lifetime doesn't match the object's lifetime →
  **Introduce Null Object** if a sentinel is unavoidable, but prefer removing the field
  from the class entirely per the two fixes above.
- **A mode/flag field that only means something during one operation** — a
  "dry run", "validate only" or "verbose" field set just before a call and meaningless
  outside it — is this smell wearing a different hat: the object's state now depends on
  which call is in flight. Prefer passing it as a parameter, or splitting the operation
  into two explicit ones.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Field | Only meaningful during | Location | Fix |
| ----- | ------------------------ | -------- | --- |
| `_currentBatchState` | `ImportBatch()` and its 3 private helpers | `datei:zeile` | Extract Class `BatchImportContext`, passed as a parameter instead of a field |

Only list rows for fields with a genuinely narrow, algorithm-scoped lifetime — a field
set in the constructor and used throughout the object's life isn't a finding. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
