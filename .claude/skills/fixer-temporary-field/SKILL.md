---
name: fixer-temporary-field
description: Apply the fix for the Temporary Field code smell — extract a field only meaningful during one algorithm into its own object instead of leaving it empty otherwise. Use when a `verifier-temporary-field` finding needs remediating, or directly asked to fix it — "fix this temporary field", "dieses nur zeitweise befuellte Feld entfernen", "extract this algorithm's temp fields into their own class".
arguments: Optional. What to fix — a `verifier-temporary-field` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Temporary Field Fixer

Applies refactoring.guru's fix for this smell: "temporary fields get their
values... only under certain circumstances. Outside of these circumstances,
they're empty." Paired with `verifier-temporary-field`, which excludes a field
set in the constructor and used throughout the object's life from counting
as a finding; this skill applies the fix only to a genuinely
algorithm-scoped field.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **A field only assigned inside one method (or a small cluster of private
  helpers) and read nowhere else** → **Extract Class**: pull that method
  plus its temporary fields into a small object created just for the one
  algorithm's duration.
- **A field standing in for what should be several method parameters**, set
  right before a chain of private calls → same fix, or **Replace Method
  with Method Object** if the whole call chain is one logical operation.
- **Conditional logic scattered around checking "is this field set right
  now"** → prefer removing the field via the two fixes above; **Introduce
  Null Object** only if a sentinel is truly unavoidable.

Never reintroduce a "sometimes meaningful" flag field as part of the fix — a
mode field that only means something during one operation ("dry run",
"validate only") is this same smell wearing a different hat. Pass it as a
parameter, or split the operation in two.
