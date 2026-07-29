---
name: verifier-lazy-class
description: Review a diff/branch for a class that doesn't do enough to earn the cost of understanding and maintaining it — a thin wrapper or leftover from a refactor whose remaining job doesn't justify its own type. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Lazy Class code smell — "lazy class check", "verdient diese Klasse noch ihre Existenz", "is this class thin enough to just inline".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Lazy Class Check

Refactoring.guru's smell: a class that doesn't earn its keep — "understanding and
maintaining classes always costs time and money," so a class reduced (by refactoring,
or by a feature that never grew as planned) to almost nothing should go away. Related
to, but narrower than, `verifier-middle-man` (a lazy class may or may not delegate
everything to one other class — if it does, that's the middle-man variant specifically).

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **A class with one field and one trivial method**, whose entire body could be a
  static helper or inlined at its call sites → **Inline Class**.
- **A class left behind after a refactor stripped out most of its responsibilities**,
  where the diff shows the class shrinking to near-nothing but not being removed →
  **Inline Class** / **Collapse Hierarchy** into whatever now holds the remaining
  responsibility.
- **A once-planned extension point that never grew** — a subclass introduced for
  variation that turned out to have exactly one implementation and no roadmap for a
  second → **Collapse Hierarchy**.
- **Don't flag intentionally thin types** — a value object, a marker type, or a single
  case with a genuinely narrow, complete job is not lazy; the finding requires the
  class costing more to navigate/maintain than it delivers.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Class | What's left in it | Location | Fix |
| ----- | -------------------- | -------- | --- |
| `MacSucheZeileMapper` | One pass-through method, superseded by the new Shared mapper | `datei:zeile` | Inline Class — fold the one method into its sole caller, then delete |

Only list rows for classes whose remaining job genuinely doesn't justify a separate
type — small-but-complete value objects and closed-set cases are not a finding. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
