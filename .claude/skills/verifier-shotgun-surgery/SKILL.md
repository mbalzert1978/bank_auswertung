---
name: verifier-shotgun-surgery
description: Review a diff/branch for one conceptual change that required touching many small edits scattered across many different classes/files — the mirror image of Divergent Change. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Shotgun Surgery code smell — "shotgun surgery check", "musste ich fuer eine Aenderung ueberall ein bisschen anfassen", "does this one change ripple across too many files".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Shotgun Surgery Check

Refactoring.guru's smell: "a single change is made to multiple classes simultaneously"
— the opposite of Divergent Change. The signal is in the diff's shape: many files, each
with a small, similar edit, all in service of one conceptual change.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **One logical change spread across many files as small, structurally similar edits**
  — e.g. adding a new enum case requires touching every switch that discriminates on it
  (this is the same underlying cause `verifier-switch-statements` flags from the
  branching-logic angle; cross-reference rather than double-counting the same root
  cause).
- **The same knowledge duplicated across files** so a rule change requires editing each
  copy → **Move Method** / **Move Field** to consolidate the knowledge into one place,
  or **Inline Class** if a thin pass-through class is what's forcing the ripple through
  an extra hop.
- **A responsibility that's correctly factored but incompletely** — the diff shows the
  edit *should* have been one method's worth of change but the current design forces
  duplicating it — is a stronger finding than an edit that's inherently cross-cutting
  (e.g. a rename that legitimately touches every call site is not this smell).

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Conceptual change | Files/classes touched | Root cause | Fix |
| ------------------ | ----------------------- | ----------- | --- |
| Add a new `Reservation` kind | 6 files: 3 switches, 2 converters, 1 presenter | Type-code duplicated instead of centralized | Move the per-kind logic onto the case itself (see `verifier-switch-statements`); consolidate to one conversion point |

Only list rows where the ripple stems from a genuine structural gap, not an inherently
cross-cutting change (renames, formatting). End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
