---
name: fixer-inappropriate-intimacy
description: Apply the fix for the Inappropriate Intimacy code smell — separate two classes that reach into each other's internals so they can change, reuse, and test in isolation. Use when a `verifier-inappropriate-intimacy` finding needs remediating, or directly asked to fix it — "fix this inappropriate intimacy", "diese enge Kopplung zwischen den Klassen aufloesen", "decouple these two classes' internals".
arguments: Optional. What to fix — a `verifier-inappropriate-intimacy` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Inappropriate Intimacy Fixer

Applies refactoring.guru's fix for this smell: "one class uses the internal
fields and methods of another class." Paired with
`verifier-inappropriate-intimacy`, which distinguishes this mutual, structural
coupling from a normal public-API collaboration; this skill applies the fix
once a genuine instance is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **Two classes reading/writing each other's non-public state** → **Move
  Method** / **Move Field** to put the shared behavior in one place, or
  **Extract Class** for the part they both need.
- **A bidirectional association either side can mutate freely** →
  **Change Bidirectional Association to Unidirectional** so only one side
  owns the relationship.
- **A class exposing internals just so a sibling can delegate through it** →
  **Hide Delegate** to remove the need for the sibling to know the internal
  shape at all.
- **A subclass reaching past its own contract into a sibling's private
  structure** where composition fits better → **Replace Delegation with
  Inheritance** (or the reverse — whichever direction actually fits here).

Never open a visibility boundary as part of the fix — no widening a member
to make the other side compile, no friend/internal grant, no reflection back
door. Route through a public contract instead; the point of the fix is to
remove the reach-through, not to legalize it.
