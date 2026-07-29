---
name: fixer-message-chains
description: Apply the fix for the Message Chains code smell — hide a navigation chain behind a method on the object the client already holds. Use when a `verifier-message-chains` finding needs remediating, or directly asked to fix it — "fix this message chain", "diese Kette von Aufrufen aufloesen", "hide this delegate chain", "law of demeter fix".
arguments: Optional. What to fix — a `verifier-message-chains` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Message Chains Fixer

Applies refactoring.guru's fix for this smell: "a client requests another
object, that object requests yet another one, and so on... the client is
dependent on navigation along the class structure." Paired with
`verifier-message-chains`, which distinguishes this from an accepted fluent
builder or collection/query pipeline; this skill applies the fix once a
genuine structural navigation chain is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **A chain of 3+ calls/property accesses reaching through intermediate
  objects** → **Hide Delegate**: expose the needed value directly on the
  object the client already holds, so the client stops knowing the path
  through the middle.
- **The same chain repeated at more than one call site** → **Extract
  Method** to name the traversal once, then **Move Method** it onto the
  root object.
- **A chain reaching through a layer boundary the codebase otherwise keeps
  closed** (an outer-layer caller navigating into a nested inner-layer shape)
  → flatten the value at that boundary instead of exposing the nested shape,
  regardless of chain length.

Never touch a fluent builder API or a collection/query pipeline used as
intended — that's an accepted idiom, not this smell.
