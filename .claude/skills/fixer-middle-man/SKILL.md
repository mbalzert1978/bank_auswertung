---
name: fixer-middle-man
description: Apply the fix for the Middle Man code smell — remove a class whose methods do nothing but forward to another class, pointing callers at the delegate directly. Use when a `verifier-middle-man` finding needs remediating, or directly asked to fix it — "fix this middle man", "diesen reinen Delegierer entfernen", "remove this pass-through wrapper".
arguments: Optional. What to fix — a `verifier-middle-man` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Middle Man Fixer

Applies refactoring.guru's fix for this smell: "a class performs only one
action, delegating work to another class." Paired with `verifier-middle-man`,
which explicitly excludes an intentional port/adapter seam that exists to
keep a dependency direction correct; this skill applies the fix only to a
delegate-only class with no such structural purpose.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **Every public method on the class is a one-line forward with no added
  value** → **Remove Middle Man**: repoint every caller at the delegate
  directly, then delete the wrapper class.
- **A near-total pass-through with one genuinely useful method** → keep the
  class for that one method only; remove the pure-forwarding methods and
  repoint their callers at the delegate.

Never remove an intentional architectural seam — a port/adapter boundary, a
facade isolating a dependency so it can be swapped or faked, an
anti-corruption layer translating a foreign model. Those delegate by design:
the indirection itself is the value, not a finding.
