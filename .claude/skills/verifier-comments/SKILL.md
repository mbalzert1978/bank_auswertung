---
name: verifier-comments
description: Review a diff/branch for comments used to explain WHAT unclear code does instead of clarifying a genuinely non-obvious WHY — the sign the code itself should be renamed/extracted/asserted rather than annotated. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Comments code smell — "comments check", "erklaert der Kommentar nur was der Code eh zeigt", "is this a WHY comment or a crutch for bad naming".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Comments Check

Refactoring.guru's smell: comments are "usually created with the best of intentions,
when the author realizes that his or her code isn't intuitive." The bar this check
applies: a comment earns its place only when the **why** is non-obvious — a constraint,
a workaround, a reason a surprising choice is correct. A comment explaining **what** the
code does is a symptom, not documentation: well-named identifiers already say that, and
where they don't, the name is the thing to fix.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **A comment that restates the next line in prose** (`// increment counter` above
  `counter++`, `// loop over reservations` above a `foreach`) — pure WHAT, adds nothing
  a reader can't already see → **delete it**, or if the intent is that the code is
  unclear, **Rename Method**/**Extract Variable**/**Extract Method** to make the name
  carry the meaning instead.
- **A block comment introducing a section of a long method** (`// now validate`,
  `// then persist`) — this is also `verifier-long-method`'s trigger for **Extract
  Method**; don't double-report, cross-reference it.
- **A comment stating an invariant that could instead be enforced/checked in code** —
  "assumes X is never null here" → **Introduce Assertion** so the invariant is checked,
  not just documented and hoped for.
- **A genuine WHY comment** — a non-obvious constraint, a workaround for a specific bug,
  a reason a seemingly-wrong-looking choice is actually correct — is **not** a finding;
  this check must not flag those, only WHAT-comments and crutches for unclear code.
- **Comments referencing the current task/fix/issue number** ("added for the batch-import
  fix") — changelog material that belongs in the commit message, where it stays tied to
  the change; in the code it goes stale the moment the next edit lands. Flag if introduced.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Comment | Why it's a smell | Location | Fix |
| ------- | ------------------ | -------- | --- |
| `// validate mac before continuing` | Restates the next `if` block; the block itself should be named | `datei:zeile` | Extract Method `ValidateMac(...)`, delete the comment |

Only list rows for WHAT-comments or unclear-code crutches — leave genuine WHY comments
alone entirely, don't even mention them as "fine." End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
