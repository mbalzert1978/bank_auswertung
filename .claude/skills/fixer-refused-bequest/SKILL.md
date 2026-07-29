---
name: fixer-refused-bequest
description: Apply the fix for the Refused Bequest code smell — replace an ill-fitting inheritance relationship with delegation or a narrower base. Use when a `verifier-refused-bequest` finding needs remediating, or directly asked to fix it — "fix this refused bequest", "diese Vererbung durch Delegation ersetzen", "this subclass shouldn't inherit all of this".
arguments: Optional. What to fix — a `verifier-refused-bequest` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Refused Bequest Fixer

Applies refactoring.guru's fix for this smell: a subclass "uses only some of
the methods and properties inherited from its parents." Paired with
`verifier-refused-bequest`, which requires genuine unused/refused/no-op
members (not one narrow, justified override) before flagging; this skill
applies the fix once a genuine instance is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **An override that raises "not supported"/"not implemented", or is a
  deliberate no-op** → **Replace Inheritance with Delegation**: hold
  a reference to the base behavior instead of inheriting it.
- **Inherited public members the subclass never calls and never expects
  callers to call through it** → **Extract Superclass** with only the
  genuinely shared members, moving the rest down to where it's actually
  used.
- **A subclass built purely to reuse one or two methods**, dragging in an
  entire base contract it doesn't otherwise want → prefer
  delegation/composition over inheriting the whole shape for a fraction of
  it.
