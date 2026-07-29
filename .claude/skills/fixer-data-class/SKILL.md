---
name: fixer-data-class
description: Apply the fix for the Data Class code smell — move behavior into an anemic Domain type instead of leaving callers to read its fields and decide externally. Use when a `verifier-data-class` finding needs remediating, or directly asked to fix it — "fix this anemic domain class", "gib dieser Klasse Verhalten", "move this logic into the domain object".
arguments: Optional. What to fix — a `verifier-data-class` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Data Class Fixer

Applies refactoring.guru's fix for this smell: a data class "contains only
fields and crude methods for accessing them... lacks independent
functionality." Paired with `verifier-data-class`, which decides what counts as
a genuine instance (Domain/aggregate types only — boundary DTOs/Response
records are explicitly out of scope there, and stay out of scope here too);
this skill applies the fix once one is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **A `Domain/` type with only auto-properties and no methods that operate on
  its own state** → **Move Method**: bring the operation that currently
  lives in a caller into the class itself (the Object-Calls-Port pattern).
- **Public setters letting external code mutate state piecemeal** →
  **Remove Setting Method** / **Encapsulate Field**, replacing free mutation
  with a behavior-bearing method that enforces the invariant.
- **A public collection field exposed for direct external mutation** →
  **Encapsulate Collection** so add/remove goes through the owning class.

Never apply these to a boundary type — a command, an event, a
request/response DTO or a serialization shape is *supposed* to be inert data
crossing a seam, not a finding.
