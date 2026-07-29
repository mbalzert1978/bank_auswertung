---
name: fixer-switch-statements
description: Apply the fix for the Switch Statements code smell — move a discriminant switched on in several places onto the type itself, or onto a closed sum type's cases. Use when a `verifier-switch-statements` finding needs remediating, or directly asked to fix it — "fix this switch statement smell", "diesen Typ-Code durch Polymorphie ersetzen", "replace this switch with polymorphism".
arguments: Optional. What to fix — a `verifier-switch-statements` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Switch Statements Fixer

Applies refactoring.guru's fix for this smell: "a complex switch operator or
sequence of if statements." Paired with `verifier-switch-statements`, which
requires the same discriminant to be switched/if-chained in more than one
location before flagging — a single, exhaustive, centrally-located match over
a closed set is the accepted pattern, not a finding; this skill applies the
fix only to a genuine multi-location instance.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **The same enum/type-code switched on in two or more separate locations**
  → **Replace Type Code with Subclasses** or **...with State/Strategy** —
  or move the branch-specific behavior onto a closed sum type's cases,
  keeping at most one exhaustive match, at the boundary where the type has to
  be converted outward.
- **A parameter used only to select which of several near-identical methods
  to run** → **Replace Parameter with Explicit Methods**.
- **A branch whose only job is "handle the absent/none case"** → **Introduce
  Null Object** instead of a null-check branch repeated at each call site.
- **Logic misplaced relative to the type it switches on** → **Extract
  Method** / **Move Method** first, to gather the scattered logic before
  converting it to polymorphism.

Leave a single, exhaustive, centrally-located match over a closed set alone
— consolidating to that shape is the fix's end state, not something to
"fix" further.
